//! Guard helpers for verified PC-local `curl` launches while offline (`NET0c3`, `NET0c4`).

use std::ffi::OsStr;
use std::io;
use std::process::Command;

/// Guard arguments (`first`, `last`) required around an offline local-only `curl` launch.
///
/// `first` (`["-q"]`) must come before every other argument so `curl` ignores `.curlrc` /
/// `_curlrc`, and `last` (`["--globoff", "--noproxy", "*", "--max-redirs", "0"]`) must come at the
/// end so the URL is taken as written (no `{}` / `[]` expansion, `NET0c5`), loopback traffic
/// bypasses proxies and no redirect is followed.
#[must_use]
pub const fn local_curl_guard_args() -> (&'static [&'static str], &'static [&'static str]) {
    (&["-q"], &["--globoff", "--noproxy", "*", "--max-redirs", "0"])
}

fn denied(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, msg)
}

fn short_opt_takes_value(ch: char) -> bool {
    matches!(
        ch,
        'E' | 'C' | 'b' | 'c' | 'd' | 'D' | 'F' | 'P' | 'H' | 'h' | 'm' | 'o' | 'Q' | 'r'
            | 'e' | 'X' | 'Y' | 'y' | 't' | 'z' | 'T' | 'u' | 'U' | 'x' | 'A' | 'w'
    )
}

fn is_refused_long_steering_opt(opt: &str) -> bool {
    opt.starts_with("--proxy")
        || opt.starts_with("--socks")
        || opt.starts_with("--dns-")
        || opt.starts_with("--doh-")
        || matches!(
            opt,
            "--url" | "--url-query" | "--request-target" | "--next"
                | "--alt-svc" | "--hsts" | "--resolve" | "--connect-to" | "--ech"
                | "--interface" | "--unix-socket" | "--abstract-unix-socket"
                | "--ipfs-gateway" | "--metalink" | "--preproxy" | "--noproxy"
                | "--ftp-port" | "--proto" | "--proto-default" | "--no-disable"
        )
}

fn long_opt_takes_value(opt: &str) -> bool {
    matches!(
        opt,
        "--abstract-unix-socket" | "--alt-svc" | "--aws-sigv4" | "--cacert" | "--capath"
            | "--cert" | "--cert-type" | "--ciphers" | "--connect-timeout"
            | "--continue-at" | "--cookie" | "--cookie-jar" | "--create-file-mode" | "--crlfile"
            | "--curves" | "--data" | "--data-ascii" | "--data-binary" | "--data-raw"
            | "--data-urlencode" | "--delegation" | "--dns-interface" | "--dns-ipv4-addr"
            | "--dns-ipv6-addr" | "--dns-servers" | "--dump-header" | "--ech"
            | "--egd-file" | "--engine" | "--etag-compare" | "--etag-save" | "--expect100-timeout"
            | "--form" | "--form-string" | "--ftp-account" | "--ftp-alternative-to-user"
            | "--ftp-method" | "--ftp-port" | "--ftp-ssl-ccc-mode" | "--happy-eyeballs-timeout-ms"
            | "--haproxy-clientip" | "--header" | "--help" | "--hostpubmd5" | "--hostpubsha256"
            | "--hsts" | "--interface" | "--ipfs-gateway" | "--ip-tos" | "--json"
            | "--keepalive-cnt" | "--keepalive-time" | "--key" | "--key-type" | "--knownhosts"
            | "--krb" | "--libcurl" | "--limit-rate" | "--local-port" | "--login-options"
            | "--mail-auth" | "--mail-from" | "--mail-rcpt" | "--max-filesize" | "--max-redirs"
            | "--max-time" | "--netrc-file" | "--noproxy" | "--oauth2-bearer" | "--output"
            | "--output-dir" | "--parallel-max" | "--parallel-max-host" | "--pass"
            | "--pinnedpubkey" | "--preproxy" | "--proto" | "--proto-default" | "--proto-redir"
            | "--proxy" | "--proxy-cacert" | "--proxy-capath" | "--proxy-cert"
            | "--proxy-cert-type" | "--proxy-ciphers" | "--proxy-crlfile" | "--proxy-header"
            | "--proxy-key" | "--proxy-key-type" | "--proxy-pass" | "--proxy-pinnedpubkey"
            | "--proxy-service-name" | "--proxy-tls13-ciphers" | "--proxy-tlsauthtype"
            | "--proxy-tlspassword" | "--proxy-tlsuser" | "--proxy-user" | "--pubkey" | "--quote"
            | "--random-file" | "--range" | "--rate" | "--referer" | "--request"
            | "--request-target" | "--retry" | "--retry-delay" | "--retry-max-time"
            | "--sasl-authzid" | "--service-name" | "--sigalgs" | "--socks4" | "--socks4a"
            | "--socks5" | "--socks5-gssapi-service" | "--socks5-hostname" | "--speed-limit"
            | "--speed-time" | "--ssl-sessions" | "--stderr" | "--telnet-option" | "--tftp-blksize"
            | "--time-cond" | "--tls13-ciphers" | "--tls-max" | "--tlsauthtype" | "--tlspassword"
            | "--tlsuser" | "--trace" | "--trace-ascii" | "--trace-config" | "--unix-socket"
            | "--upload-file" | "--upload-flags" | "--url-query" | "--user" | "--user-agent"
            | "--variable" | "--vlan-priority" | "--write-out"
    )
}

fn validate_offline_local_args(args: &[&OsStr]) -> io::Result<()> {
    let mut i = 0;
    let mut stdin_config_seen = false;
    while i < args.len() {
        let Some(arg) = args[i].to_str() else {
            return Err(denied("non-utf8 curl option"));
        };
        if arg == "--" {
            // Also as the last argument: the guards appended after it would be read as URLs (`NET0c5`).
            return Err(denied("positional url refused"));
        }
        if arg.starts_with("--") {
            let (raw_opt, inline_val) = match arg.split_once('=') {
                Some((k, v)) => (k, Some(v)),
                None => (arg, None),
            };
            let expanded;
            let opt = if let Some(rest) = raw_opt.strip_prefix("--expand-") {
                expanded = format!("--{rest}");
                expanded.as_str()
            } else {
                raw_opt
            };
            if matches!(
                opt,
                "--location" | "--location-trusted" | "--follow" | "--max-redirs" | "--proto-redir"
            ) {
                return Err(denied("redirects refused"));
            }
            if opt == "--config" {
                if raw_opt != "--config"
                    || inline_val.is_some()
                    || stdin_config_seen
                    || args.get(i + 1).and_then(|v| v.to_str()) != Some("-")
                {
                    return Err(denied("config file refused"));
                }
                stdin_config_seen = true;
                i += 2;
                continue;
            }
            if is_refused_long_steering_opt(opt) {
                return Err(denied("steering option refused"));
            }
            if long_opt_takes_value(opt) && inline_val.is_none() {
                if i + 1 >= args.len() {
                    return Err(denied("missing option value"));
                }
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if let Some(rest) = arg.strip_prefix('-') {
            if !rest.is_empty() {
                let mut consumes_next = false;
                for (idx, ch) in rest.char_indices() {
                    if ch == 'L' {
                        return Err(denied("redirects refused"));
                    }
                    if ch == 'K' {
                        return Err(denied("config file refused"));
                    }
                    if matches!(ch, ':' | 'x' | 'U' | 'p' | 'P') {
                        return Err(denied("steering option refused"));
                    }
                    if short_opt_takes_value(ch) {
                        consumes_next = idx + ch.len_utf8() == rest.len();
                        break;
                    }
                }
                if consumes_next {
                    if i + 1 >= args.len() {
                        return Err(denied("missing option value"));
                    }
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
        }
        return Err(denied("positional url refused"));
    }
    if !stdin_config_seen {
        return Err(denied("missing stdin config"));
    }
    Ok(())
}

pub(super) fn prepare_offline_local_curl(
    cmd: Command,
    url: &str,
    stdin_bytes: &[u8],
) -> io::Result<Command> {
    if !super::is_local_target(url) {
        return Err(denied("network disabled"));
    }
    if stdin_bytes != crate::curl_stdin::curl_url_config(url).as_bytes() {
        return Err(denied("stdin config mismatch"));
    }
    let orig_args: Vec<&OsStr> = cmd.get_args().collect();
    validate_offline_local_args(&orig_args)?;

    // Rebuild `Command` so `first` (`-q`) precedes every caller argument and `last` trails them.
    // Carried over from `cmd`: program (`get_program`), arguments (`get_args`), explicit per-var
    // `env` / `env_remove` overrides (`get_envs`), and working directory (`get_current_dir`).
    // Not carried over (`std::process::Command` exposes no getters for them): `env_clear()`,
    // pre-set stdio handles (`spawn_with_stdin` pipes all three), and Windows `raw_arg` or
    // `creation_flags` other than `CREATE_NO_WINDOW` (`0x0800_0000`) set below.
    let (first, last) = local_curl_guard_args();
    let mut rebuilt = Command::new(cmd.get_program());
    rebuilt.args(first);
    rebuilt.args(orig_args);
    rebuilt.args(last);
    for (k, v) in cmd.get_envs() {
        match v {
            Some(val) => {
                rebuilt.env(k, val);
            }
            None => {
                rebuilt.env_remove(k);
            }
        }
    }
    if let Some(dir) = cmd.get_current_dir() {
        rebuilt.current_dir(dir);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        rebuilt.creation_flags(0x0800_0000);
    }
    Ok(rebuilt)
}

#[cfg(test)]
#[path = "network_local_tests.rs"]
mod network_local_tests;

#[cfg(test)]
#[path = "network_local_net0c4_tests.rs"]
mod network_local_net0c4_tests;

#[cfg(test)]
#[path = "network_local_net0c5_tests.rs"]
mod network_local_net0c5_tests;
