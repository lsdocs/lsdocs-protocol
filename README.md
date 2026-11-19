# lsdocs-protocol

Part of [lsdocs](https://github.com/lsdocs), a static-site generator whose
parsers, transformers and renderers are plugins.

The wire protocol between the lsdocs host and its plugins:

- `spec/wire.md`: the specification (envelope, framing, the methods,
  syntax extensions, configuration, versions). A plugin written from it
  alone, with no SDK, works (lsdocs' `examples/plugins/txt-parser`).
- `crates/protocol` (`lsdocs-protocol`): the message types, the
  `Request` / `Notification` traits and the msgpack codec.
- `crates/config` (`lsdocs-config`): `lsdocs.toml` parsing and validation
  (no I/O), whose types the protocol carries.
- `schema/` and `fixtures/`: JSON Schemas and golden msgpack/JSON
  messages, generated from the types by `tools/gen` (`nur gen`). Each
  release attaches `schema.tar.gz`; https://lsdocs.github.io serves them.

`PROTOCOL_VERSION` is `"1.0-rc.0"`, a release candidate that must match
exactly until 1.0 (see "Versions" in the spec).

## Known limitations

- Transformers receive whole documents and return whole documents.
- Site-wide data (the navigation) is sent with every page.

## TODO

- [ ] Send transformers only the subtrees they select, and accept patches
      back (decide with benchmarks, before 1.0).
- [ ] Share site-wide data (the navigation) once instead of per page
      (open; when the navigation's cost matters).
