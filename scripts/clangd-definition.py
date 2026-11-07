#!/usr/bin/env python3
"""Resolve one C++ source position using the current project's clangd database."""
import argparse
import json
import os
from pathlib import Path
import queue
import shutil
import subprocess
import sys
import threading
import time
from urllib.parse import urlsplit
from urllib.request import url2pathname


class Client:
    def __init__(self, executable, root, timeout):
        self.deadline = time.monotonic() + timeout
        self.messages = queue.Queue()
        self.diagnostics = {}
        self.next_id = 0
        self.proc = subprocess.Popen(
            [executable, "--background-index=false", "-j=2", "--log=error",
             f"--compile-commands-dir={root / 'build'}"],
            cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        self.reader = threading.Thread(target=self.read_messages, daemon=True)
        self.reader.start()
        # Also bound a blocked stdin write, not just waits for RPC replies.
        self.watchdog = threading.Timer(timeout, self.stop_process)
        self.watchdog.daemon = True
        self.watchdog.start()

    def stop_process(self):
        if self.proc.poll() is None:
            try:
                self.proc.kill()
            except ProcessLookupError:
                pass

    def read_messages(self):
        try:
            while True:
                headers = {}
                while True:
                    line = self.proc.stdout.readline()
                    if not line:
                        raise RuntimeError("clangd closed its output")
                    if line in (b"\r\n", b"\n"):
                        break
                    name, value = line.decode("ascii").split(":", 1)
                    headers[name.lower()] = value.strip()
                length = int(headers["content-length"])
                if not 0 < length <= 32 * 1024 * 1024:
                    raise RuntimeError("invalid LSP message length")
                body = self.proc.stdout.read(length)
                if len(body) != length:
                    raise RuntimeError("incomplete LSP message")
                self.messages.put(json.loads(body))
        except Exception as error:
            self.messages.put(error)

    def send(self, message):
        body = json.dumps({"jsonrpc": "2.0", **message}, ensure_ascii=False).encode("utf-8")
        self.proc.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode("ascii") + body)
        self.proc.stdin.flush()

    def receive(self):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("clangd request exceeded the time limit")
        try:
            message = self.messages.get(timeout=remaining)
        except queue.Empty:
            raise TimeoutError("clangd request exceeded the time limit") from None
        if isinstance(message, Exception):
            raise message
        method = message.get("method")
        if method and "id" in message:
            params = message.get("params", {})
            if method == "workspace/configuration":
                result = [{} for _ in params.get("items", [])]
            elif method == "workspace/applyEdit":
                result = {"applied": False}
            else:
                result = None
            self.send({"id": message["id"], "result": result})
        elif method == "textDocument/publishDiagnostics":
            params = message["params"]
            self.diagnostics[params["uri"]] = params.get("diagnostics", [])
        return message

    def request(self, method, params):
        self.next_id += 1
        request_id = self.next_id
        self.send({"id": request_id, "method": method, "params": params})
        while True:
            message = self.receive()
            if "method" not in message and message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(f"{method}: {message['error']}")
                return message.get("result")

    def close(self):
        # This helper owns this one clangd process; never touch existing editors.
        self.watchdog.cancel()
        self.stop_process()
        self.proc.wait(timeout=2)
        self.reader.join(timeout=1)
        self.proc.stdin.close()
        self.proc.stdout.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", help="repository-relative C++ source/header")
    parser.add_argument("line", type=int, help="1-based line")
    parser.add_argument("column", type=int, help="1-based UTF-16 column on the symbol")
    parser.add_argument("--timeout", type=float, default=45, help="total request budget, 1-55 seconds")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    client = None
    try:
        if not 1 <= args.timeout <= 55:
            raise ValueError("--timeout must be between 1 and 55 seconds")
        if Path(args.file).is_absolute():
            raise ValueError("file must be repository-relative")
        source = (root / args.file).resolve()
        if not source.is_relative_to(root) or not source.is_file():
            raise ValueError("file must exist inside this repository")
        if source.suffix.lower() not in {".cpp", ".cc", ".cxx", ".c", ".h", ".hpp", ".hxx"}:
            raise ValueError("file must be C++/C source or a header")
        text = source.read_text(encoding="utf-8-sig")
        lines = text.split("\n")
        if not 1 <= args.line <= len(lines):
            raise ValueError("line is outside the source file")
        line = lines[args.line - 1].rstrip("\r")
        if not 1 <= args.column <= len(line.encode("utf-16-le")) // 2:
            raise ValueError("column must point to a character on the selected line")
        if not (root / "build/compile_commands.json").is_file():
            raise RuntimeError("missing compiler database; run scripts/gen-compile-commands.ps1")
        executable = shutil.which("clangd")
        if not executable:
            candidate = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "LLVM/bin/clangd.exe"
            if candidate.is_file():
                executable = str(candidate)
        if not executable:
            raise RuntimeError("clangd not found on PATH or under Program Files/LLVM")
        client = Client(executable, root, args.timeout)
        uri = source.as_uri()
        client.request("initialize", {
            "processId": os.getpid(), "rootUri": root.as_uri(),
            "workspaceFolders": [{"uri": root.as_uri(), "name": root.name}],
            "capabilities": {"general": {"positionEncodings": ["utf-16"]},
                             "offsetEncoding": ["utf-16"]}})
        client.send({"method": "initialized", "params": {}})
        client.send({"method": "textDocument/didOpen", "params": {"textDocument": {
            "uri": uri, "languageId": "cpp", "version": 1, "text": text}}})
        while uri not in client.diagnostics:
            client.receive()
        locations = client.request("textDocument/definition", {
            "textDocument": {"uri": uri},
            "position": {"line": args.line - 1, "character": args.column - 1}})
        if not locations:
            raise RuntimeError("no declaration/definition returned at this position")
        targets = []
        for location in locations if isinstance(locations, list) else [locations]:
            parsed = urlsplit(location.get("targetUri", location.get("uri", "")))
            if parsed.scheme != "file":
                continue
            path = url2pathname(("//" + parsed.netloc if parsed.netloc else "") + parsed.path)
            start = location.get("targetSelectionRange", location.get("range"))["start"]
            target = {"file": str(Path(path)), "line": start["line"] + 1,
                      "column": start["character"] + 1}
            if target not in targets:
                targets.append(target)
        if not targets:
            raise RuntimeError("clangd returned no usable file locations")
        print(json.dumps({"source": str(source), "line": args.line, "column": args.column,
                          "position_encoding": "utf-16", "locations": targets,
                          "diagnostic_error_count": sum(d.get("severity") == 1
                                                        for d in client.diagnostics[uri])},
                         ensure_ascii=False, indent=2))
        return 0
    except (OSError, ValueError, RuntimeError, TimeoutError) as error:
        print(f"clangd-definition: {error}", file=sys.stderr)
        return 1
    finally:
        if client:
            client.close()


if __name__ == "__main__":
    sys.exit(main())
