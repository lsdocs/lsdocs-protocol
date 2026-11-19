# AGENTS.md

Guidance for AI agents (and humans) working in lsdocs-protocol, one repository of
the lsdocs organization (https://github.com/lsdocs). It was split from the
lsdocs monorepo at commit d71dfa3; its history before that is the
monorepo's, filtered to these files. README.md says what it is and lists
its known limitations and TODO items.

## Working across repositories

Other lsdocs repositories are git dependencies
(`{ git = "https://github.com/lsdocs/<repo>", branch = "main" }`);
`Cargo.lock` pins their revisions (`nur bump [crate]` moves them).

- `nur link ../<repo>` builds against a local checkout (a `[patch]` in the
  untracked `.cargo/config.toml`); `nur unlink` undoes it.
- With `LSDOCS_ORG_DIR` set (e.g. `C:/dev`), Cargo fetches the
  organization's repositories from the local clones there (their
  committed `main`) instead of GitHub.
- `nur check-dupes` fails if a crate of the organization is built twice;
  `nur check-git-deps` fails on a dependency that is not a path inside
  this repository or a git dependency on the repository holding it
  (`scripts/org.nuon`), and on a committed `[patch]` table. `nur msrv`
  checks each crate on its own `rust-version`; `nur deny` runs
  cargo-deny (both are CI jobs of their own).

## Dev tasks

Dev tasks are nushell scripts run with `nur` (`nurfile`); `nur --list`
shows them. `nur ci` runs what CI runs: run it before committing. Tools:
`mise install` (see `mise.toml`); Rust from rustup.

## Conventions

- Commits: Conventional Commits, `type(scope): summary`.
- Line endings: LF everywhere; scripts that write files write LF.
- Comments and docs: ASCII only.
- MSRV is per crate (`rust-version`); the workspace's is the base.
- Dependencies are declared in `[workspace.dependencies]` and used with
  `workspace = true`.
- Lints: the workspace lints apply to every crate; `missing_docs` is on;
  `unsafe_code` is denied.
- No panics outside tests: clippy denies `unwrap_used`, `expect_used` and
  `panic` in library, binary and example code. Tests opt in per module
  (`#![allow(clippy::unwrap_used)]`).
- Licensing: `Apache-2.0 OR MIT`. Nothing is published.
- Record what is left open in README.md's TODO section, and what the code
  does not do (or does differently) under Known limitations.

## This repository

| Path | What |
|---|---|
| `crates/protocol` | `lsdocs-protocol`: message types, `Request`/`Notification` traits, `msgpack` codec. |
| `crates/config` | `lsdocs-config`: `lsdocs.toml` parsing, validation, env/CLI overrides. No I/O. |
| `tools/gen` | `lsdocs-gen`: generates `schema/` (JSON Schemas) and `fixtures/` (golden msgpack + JSON). |
| `spec/wire.md` | The wire protocol's specification. |

Tasks: `nur gen` regenerates `schema/` and `fixtures/` (never hand-edit
them; a test fails when they are stale); `nur test fixtures` decodes every
fixture with nushell's msgpack reader; `nur schema archive` makes the
archive a release attaches. A release is made by pushing a `v*` tag
(`.github/workflows/release.yml`).

## Wire format rules

These are what other languages rely on; breaking them breaks plugins.

- Encode with `lsdocs_protocol::msgpack::encode`. Structs are maps keyed by
  field name, never positional arrays.
- Field names are `snake_case`. Optional fields are omitted when empty,
  and readers ignore unknown fields (except in `lsdocs.toml`, which
  rejects them).
- Document nodes are internally tagged: `{"type": "heading", ...}`.
- Binary data is msgpack `bin` (`serde_bytes`). No msgpack `ext` types.
- msgpack-rpc params are an argument array: `[]` or `[params]`.
- `PROTOCOL_VERSION` is `"1.0-rc.0"` and must match exactly until 1.0.
  After 1.0, changes within a major version are additive only.

After changing protocol types: `nur gen`, check the fixture diff, and
update `spec/wire.md` if the envelope or semantics changed.
