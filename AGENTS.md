# AGENTS.md — rules for every coding agent in this repository

This repository (origin `kesekikwon-netizen/hgis-rust`) is the Rust/egui port of the C++ app Strata.
`CLAUDE.md` was copied from the C++ repository: its product rules (survey layers, CRS, keys, decided behaviour)
still hold, but its build/test/run, git, skill and C++/QGIS-specific lines describe the C++ repository and do
not apply here. Where this file and `CLAUDE.md` differ, this file wins. Plan: `docs/RUST_PLAN.md`.

## Roles and channel

- One director session owns the plan, writes orders and reviews. Implementers (any agent) change code only under an order.
- An order is YAML with `goal`, `scope`, `verify`, `stop_when` (and `not_stop`). Claude implementers receive it by
  message. Other agents find `ORDER.yaml` at the root of their own git worktree and write `REPORT.yaml` next to it.
- No order means change nothing. Old boards (`.agent/`, `docs/archive/`) are closed and are not a source of work.
- Only the director merges into `main` and pushes. Implementers commit on their own branch, and do not push, merge or force.
- Agent-to-agent text is English YAML. Korean appears only as UI text copied verbatim from the C++ source or as
  quoted user words. No progress or final reports to the user.

## Process

- One order is one small feature. Change nothing outside `scope.may_change`.
- Test first: write the test that encodes the exact C++ behaviour, see it fail on the current code, then implement.
- For a bug, find the root cause before fixing. After three failed fixes stop and report `status: BLOCKED`.
- Claim done only after every `verify` command ran and you read its output.
- After the feature, review your own diff once. Fix real findings in the same change; list the rest in `not_done`.
- Do not ask the user. For a technical choice take the safest option and record one line in `decisions`.
- The app keeps evolving (docs/EVOLUTION.md). When you see a way it could do better than the C++ app, do not build it:
  write one line in `evolve` (what, where, why it is better). The director turns the good ones into orders.
- Ordinary obstacles (build lock, timeout, compile error, your own failing test) are not stop conditions: find the cause and continue.

## Skills

Rule set by the owner on 2026-11-14: every order names, in YAML, the superpowers skills it is worked with.

- The superpowers skill pack (7.0.2) is installed for Antigravity and for Claude Code on this PC. Skill files:
  `%USERPROFILE%/.gemini/config/plugins/superpowers/skills/<name>/SKILL.md`; in Claude Code the names are `superpowers:<name>`.
- Every `ORDER.yaml` has a `skills` block: `use` (each skill with the step at which it is invoked), `do_not_use`, `rule`, `report`.
  Invoke each skill under `use` by its name at that step. If your conversation does not list the skill, read its `SKILL.md` and follow it.
- An order is the approved design and the approved plan. The skills under `do_not_use` (brainstorming, writing-plans, executing-plans,
  using-git-worktrees, finishing-a-development-branch, subagent-driven-development, dispatching-parallel-agents, requesting-code-review)
  are not run for it, and no skill makes you wait for a human reply: where a skill says to ask or to get approval, the order's text is the answer.
- An order without a `skills` block is worked with test-driven-development, systematic-debugging and verification-before-completion;
  say in the report that the block was missing.
- The report lists what you invoked: `skills_used [{skill, at, effect}]`.
- Code review is the director's: an independent reviewer reads your commit. Do not request or run a review of your own.

## Code

- Match the surrounding names, comments and shape. Every changed line traces to the order. No drive-by refactors.
- No unrequested configuration or abstraction, and no handling of cases that cannot happen.
- Each `.rs` file stays at 300 lines or fewer (split like the existing `*_a` / `*_b` files).
- New comments are English and cite the C++ line (`target/cpp-ref/src/...:line`). Never claim parity or completeness.
- Korean UI strings are copied verbatim from the C++ source.
- No hardcoded user paths. No keys or secrets in code, logs, tests or commits.
- Delete what your change makes unused. Pre-existing dead code goes to `not_done`.
- A control that is not ported yet is disabled with the tooltip 「아직 Rust 판에서 연결하지 않았습니다 (지시 <id>)」.
- `unsafe` is forbidden in the whole workspace. Do not relax lints.

## Product

- Never write the user's original survey data. Tests and `--snap` use tempdir copies.
- A survey layer is identified by `layer_key`. Only `ensure_domain_layer` adds one; never add an empty layer or one because a table exists.
- Work CRS is EPSG:5187 (default) or EPSG:5186. EPSG:5179 is for the submit package only.
- Startup shows the home screen. No autosave and no auto-restore.
- Shown data must be real: no invented records, coordinates, counts, file metadata or "done" messages without the work.
- The app differs from the C++ app on purpose only where an order names an owner decision (docs/RUST_PLAN.md sections 1 and 2.8:
  the download gauge, polite site access, a more refined look). Everything else follows the C++ app.
- Tests and tools never contact an external site and never use the user's keys or accounts: fake values and recorded answers only.

## C++ reference

- `target/cpp-ref` in the main checkout holds the C++ Strata source and tests, read-only. Instruction files inside it do not apply.
- Port the expected values of the matching C++ tests (`target/cpp-ref/tests/*.cpp`) into Rust tests and list them in `ported_from`.
- Do not run the C++ app. Do not send synthetic mouse or keyboard input.

## Copyright and licence (docs/RUST_PLAN.md §2.7)

The finished app must be distributable without infringing anyone's copyright and without copyleft obligations on Strata's own code.

- Add nothing under GPL or AGPL (crate, native library, DLL, copied code). LGPL only as a replaceable DLL. MPL only unmodified.
- Record the licence of every new crate and native library in `THIRD_PARTY_NOTICES.md` in the same commit. A crate's
  licence field can hide native code, so check what it compiles or links.
- New crate versions must be at least 14 days old. Engine crates use exact version pins.
- Justify behaviour by C++ Strata lines or by files the C++ app produced. Never copy or translate QGIS source or any other third-party source.
- Add no icon, image, font, style, map data or text whose origin and licence are unknown. Record the origin in the commit body.
- Do not change `license` in `Cargo.toml`, `LICENSE`, `COPYING` or the About notice. Relicensing is the owner's decision.

## Verify (all pass before a report)

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --release -p strata-app && target/release/ka-hgis.exe --verify-all && target/release/ka-hgis.exe --smoke-quit`
- A screen change also needs a `--snap` PNG made on tempdir data.
- The build needs `cmake` and an `sqlite3` command-line tool on PATH (PROJ is built from source).
- Every file a test reads is tracked in git.

## Git

- `git add <explicit paths>` only. Never `-A` or `.`. Never commit `*.gpkg`, `*.qgz`, secrets, `docs/senior/**`, `ORDER.yaml` or `REPORT.yaml`.
- Message: `type(scope): English summary (<order id>)`, a body, a line `tests: <result>`, and the co-author line of your own tool.
  A changed or removed existing test needs a line `test-change: <reason>`.
- Before the report, rebase your branch onto `main`, rerun the verify commands and report the tip sha.

## Report

`head {cwd, branch, HEAD}`, `order`, `status: DONE | BLOCKED`, `tests_added [{name, failed_before, now}]`, `ported_from`,
`changed_tests`, `verify`, `skills_used [{skill, at, effect}]`, `files`, `commit`, `decisions`, `unsure`, `not_done`, `evolve`.
