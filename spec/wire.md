# lsdocs wire protocol

Status: **release candidate, protocol version `1.0-rc.0`**. See
[Versions](#versions) for what may still change.

This document describes the bytes exchanged between the lsdocs host and a
plugin. It is enough to write a plugin in any language without the official
bindings. The message *contents* (params and results) are described by the
JSON Schemas in [`schema/protocol/`](../schema/protocol), the document tree
by [`schema/doctree/document.json`](../schema/doctree/document.json), and
[`fixtures/`](../fixtures) has an encoded example of each (MessagePack, with
a JSON twin), for testing a decoder.

A plugin is a program that answers requests for one or more build phases:
it **parses** source files into document trees, **transforms** trees, or
**renders** them to output files. The host discovers files, runs the
phases in order, and writes the output.

## Versions

The protocol version is `MAJOR.MINOR`, with `-rc.N` appended for a release
candidate (`1.0-rc.0`). The host and the plugin each state theirs in the
handshake; the host ends the session if they are not compatible:

- Two release versions are compatible if their major versions are equal.
- A release candidate is compatible only with the very same version.
  Anything may change between candidates; plugins built against one are
  rebuilt for the next and for the release.

Within a major version, changes only add things older peers can ignore:

- new optional fields (receivers ignore fields they do not know, and treat
  a missing optional field as absent);
- new optional capability fields, and new methods or notifications that are
  only sent to a peer that announced support for them;
- new diagnostic codes, new log levels' meanings within the existing set.

These need a new major version: removing or renaming a method or field,
changing a field's type or meaning, making an optional field required,
and adding a node kind to the document tree (older plugins would fail to
decode it; plugin-specific content uses `custom` nodes instead).

The schemas and fixtures in this repository describe the version stated
above.

## Running a plugin

A site's `lsdocs.toml` lists its plugins in order:

```toml
[[plugins]]
name = "mermaid"                     # used in logs and diagnostics
command = ["./bin/mermaid-plugin"]   # program and arguments
transport = "stdio"                  # or "pipe"; default "stdio"
files = ["diagrams/**"]              # optional: source globs it handles
options = { theme = "dark" }         # optional: passed in `initialize`
```

`command` may be replaced by `runtime = "python"` with `module` (run as
`python -m <module>`; the interpreter is `LSDOCS_PYTHON` if set) or
`runtime = "node"` with `entry` (run as `node <entry>`; `LSDOCS_NODE` if
set). First-party plugins use `builtin = "<name>"` and run inside the host.

The host starts the program with the site's directory (where `lsdocs.toml`
is) as its working directory. The plugin's stderr is passed through to the
user, so it is the place for stray output. The plugin has 30 seconds to
answer `initialize`, and 5 seconds to exit after `exit` before it is
killed.

## Transports

A plugin talks to the host over one of:

| Transport | How it is set up |
|---|---|
| `stdio` (default) | The host launches the plugin and uses its stdin and stdout. |
| `pipe` | The host creates a named pipe (Windows) or unix socket and launches the plugin with its name in the `LSDOCS_PIPE` environment variable. The plugin connects to it. |

**With `stdio`, stdout belongs to the protocol.** Anything else written to
stdout corrupts the stream and ends the connection. Plugins must send logs
and stray output to stderr (or use the `log` notification). The official
bindings redirect stdout to stderr for you.

## Framing

The stream is a sequence of frames. Each frame is:

```
+----------------------+---------------------------+
| length: u32, big end | payload: `length` bytes   |
+----------------------+---------------------------+
```

The payload is exactly one MessagePack value: a message, below. The default
maximum payload is 64 MiB. A receiver drops the connection when a frame
exceeds its limit, since the stream cannot be resynchronized.

## Messages

Messages follow [msgpack-rpc](https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md):

| Message | Encoding |
|---|---|
| Request | `[0, id, method, params]` |
| Response | `[1, id, error, result]` |
| Notification | `[2, method, params]` |

- `id` is an unsigned 32-bit integer chosen by the sender of the request,
  unique among that sender's requests still awaiting a response. Each side
  numbers its own requests independently.
- `method` is a string, e.g. `"parse"` or `"transform/collect"`.
- `params` is an **array of arguments**, as msgpack-rpc requires. Every
  lsdocs method takes either no argument (`[]`, for methods whose params are
  empty) or exactly one: the params map described by the method's schema.
- A response has `error` = nil and the `result` on success, or an error map
  and `result` = nil on failure. A method with no result returns nil.

Either side may send requests and notifications at any time after the
handshake. Responses may arrive in any order: the host sends a plugin up
to its `max_concurrency` requests at once (by default, one per CPU).

### Encoding rules

These keep messages readable from any language and let fields be added
without breaking older peers:

- Structs are **maps keyed by field name**, never positional arrays.
- Field names are `snake_case`.
- Optional fields are omitted when absent. Receivers must treat a missing
  optional field as absent and **ignore unknown fields**.
- Enums without data are strings (`"warning"`). Document nodes are maps with
  a `"type"` field naming the node kind.
- Binary data (render output, extra files) is MessagePack `bin`.
- MessagePack extension types are not used.

## Methods

Params and results are in `schema/protocol/<name>.params.json` and
`<name>.result.json`, where `<name>` is the method with `/` replaced by `.`
(`transform/collect` is `transform.collect`).

| Method | Kind | Direction | Params | Result |
|---|---|---|---|---|
| `initialize` | request | host -> plugin | `initialize.params` | `initialize.result` |
| `initialized` | notification | host -> plugin | none | |
| `parse` | request | host -> plugin | `parse.params` | `parse.result` |
| `transform/collect` | request | host -> plugin | `transform.collect.params` | `transform.collect.result` |
| `transform/collected` | request | host -> plugin | `transform.collected.params` | `transform.collected.result` |
| `transform/apply` | request | host -> plugin | `transform.apply.params` | `transform.apply.result` |
| `render` | request | host -> plugin | `render.params` | `render.result` |
| `render/site` | request | host -> plugin | `render.site.params` | `render.site.result` |
| `site/parse` | request | plugin -> host | `site.parse.params` | `site.parse.result` |
| `shutdown` | request | host -> plugin | none | nil |
| `exit` | notification | host -> plugin | none | |
| `$/cancel` | notification | either | `cancel.params` | |
| `log` | notification | plugin -> host | `log.params` | |
| `progress` | notification | plugin -> host | `progress.params` | |
| `diagnostics` | notification | plugin -> host | `diagnostics.params` | |

A plugin answers a request it does not handle with error -32601.

### Capabilities

The `initialize` result says which phases the plugin takes part in. A
phase the plugin does not support is omitted:

| Capability | Fields | Meaning |
|---|---|---|
| `parse` | `extensions` | File extensions (without the dot) the plugin parses. The first parser in config order that takes a file's extension (and whose `files` globs match) parses it. Files no parser takes are copied to the output. |
| `transform` | `nodes`, `collect` | `nodes`: node kinds (`"heading"`, `"custom:mermaid"`) the plugin looks at; it is only sent documents containing one, or every document if empty. `collect`: whether it takes part in the collect pass. |
| `render` | `formats`, `site` | Output formats (`"html"`). The first renderer renders every document in its first format. `site`: whether it wants `render/site`. |
| `max_concurrency` | | Most requests it handles at once. |
| `cache` | | See [Caching](#caching). |
| `syntax` | `directives`, `roles` | Directives and roles the plugin gives meaning to; see [Syntax extensions](#syntax-extensions). |

## Lifecycle

1. Host -> plugin request `initialize`, with the host's protocol version,
   the site (title, base URL, the site and source directories, every source
   file) and the plugin's `options`. The plugin answers with its protocol
   version, name, version and capabilities. The host ends the connection
   if the versions are not compatible.
2. Host -> plugin notification `initialized`.
3. The build, one phase at a time:
   1. `parse` for each file the plugin parses: its path (relative to the
      source directory, `/` separators), its text, and the `syntax` every
      plugin declared. The result is the document and any diagnostics.
   2. The host then normalizes every document (see
      [Document shape](#document-shape)).
   3. Transformers run one at a time, in config order. For a plugin that
      collects: `transform/collect` for each document it is sent, then once
      `transform/collected` with everything collected (a map from document
      source to the collected data). Its result may hold documents the
      plugin generates from that data (an index page, say), each with a
      `source` path no other document has: the host normalizes them and
      adds them to the site, so they go through the apply pass, the later
      transformers and rendering like the others. A generated document
      whose path is taken (or not a safe relative path) is an error. Then
      `transform/apply` for each document, which returns the document,
      changed or not; its `collected` field is null over the wire (the
      official SDKs fill it in from `transform/collected`).
   4. `render` for each document, then, once after every document is
      rendered, `render/site` if the render capability has `site: true`. Its
      result lists extra files, such as a sitemap.

   The plugin may send `log`, `progress` and `diagnostics` notifications at
   any time. While it handles a phase request it may send `site/parse`
   requests: the host parses the text with the first parser (in config
   order) taking files of the given `extension`, giving it the site's
   syntax extensions, and answers with that parser's result, not
   normalized. A plugin uses it for text in a markup it does not parse
   itself, such as doc comments. (A plugin that parses and asks for its
   own extension while at its `max_concurrency` would wait forever.)
4. Host -> plugin request `shutdown`, then notification `exit`. The plugin
   exits; closing the stream also ends the session.

A host may keep a session open across several builds (`lsdocs serve`
does): phase requests then repeat after the first build, without a new
`initialize`. When source files are added or removed, the host restarts
the plugins, since `initialize` lists the files.

## Document shape

The node kinds and their fields are in `schema/doctree/document.json`.
Whatever the source format, parsers produce the same shape, so any
transformer or renderer works with any parser:

- Content under a title is wrapped in a `section` whose first child is the
  title `heading`; subsections nest inside it. The section's `id` is its
  anchor. A `heading` has an `id` only when it is not a section title.
- Parsers set only ids the author wrote. After parsing, the host gives
  every other section, and every heading that is not a section title, an
  id: the slug of its title text, made unique within the document.
  - Slug: lowercase the text; keep letters and digits (Unicode
    alphanumerics); replace each run of other characters with one `-`; drop
    `-` at either end. An empty slug becomes `section`.
  - Unique: ids the author wrote (on sections, headings, figures and
    anchors) are reserved first. A slug already taken gets `-1`, `-2`, ...
    appended, the first free one winning, in document order.
  - An id the author wrote twice in one document is a `duplicate-id`
    warning.
- The host also sets `target` on links without one whose `url` is a
  relative path to a parsed document (`target.document`, plus
  `target.anchor` from any `#fragment`) or a bare `#fragment`
  (`target.anchor` only).
- Common constructs have core kinds: `admonition`, `definition_list`
  (`definition_term` and `definition_description` children),
  `footnote_reference` / `footnote_definition`, `math`, `figure` (with an
  optional `caption` last), and `anchor` (an id with no content). Anything
  else is `custom` with a plugin-defined `kind`.
- A `link` to something in the site may carry a `target`: `document`
  (source path) and/or `anchor`, or an unresolved `label`. Renderers use the
  target, not the `url`, to build the output link.
- `span`s are byte offsets into the document's source text, `end`
  exclusive.

## Syntax extensions

Directives and roles (Sphinx's `versionadded`, `ref`, ...) are syntax:
only a parser can find them. Their meaning belongs to a transformer. A
plugin declares the ones it handles in its `syntax` capability; the host
merges every plugin's declarations (the first plugin in config order wins
a name) and sends them to parsers in each `parse` request's `syntax`
field. A parser that supports syntax extensions reads them with its
format's syntax and makes the nodes below, which the declaring plugin
transforms. A parser that does not ignores the field.

A directive declaration (`schema/protocol/initialize.result.json`):

| Field | Default | Meaning |
|---|---|---|
| `name` | | Lower case, e.g. `"versionadded"`. |
| `required_arguments`, `optional_arguments` | 0 | How many arguments it takes. |
| `final_argument_whitespace` | false | Whether the last argument takes the rest of the text, spaces included. |
| `argument` | `"text"` | `"inline"`: the argument is also parsed as inline markup. |
| `options` | `{}` | Option names: `"flag"` (no value), `"value"` (may be empty) or `"required"` (a value). |
| `content` | `"none"` | `"blocks"`: parsed as block markup; `"text"`: kept as written (code, a list of names). |

A role declaration has only a `name`.

What parsers make:

- A directive is a `custom` node whose `kind` is its name, with the
  attributes `argument` (the arguments as one string, if any), `options`
  (a map from name to value: `true` for a flag, else the text) and, for
  text content, `content` (the text, without the trailing newline). Its
  children are its parsed content; for an inline argument, the first child
  is a `custom` node of kind `directive_argument` holding the parsed
  argument. Problems (an unknown option, a missing argument, content where
  none is allowed) are warnings; the reStructuredText parser reports them
  as docutils does.
- A role is a `custom` node of kind `role` with the attributes `name` and
  `text` (backslash escapes removed), and the text as its one `text`
  child, so plain-text views of it (titles, ids) are right.

The syntax per format: in reStructuredText, `.. name:: argument`, option
field lines and indented content, and `` :name:`text` ``. In Markdown,
MyST's: a fenced block whose info string starts with `{name}` (the rest is
the argument; leading `:name: value` lines or a `---` YAML block are
options; the rest is content), and `` {name}`text` ``.

A directive nobody declared stays a `custom` node of its name with
`argument`, `options` and `content` as text, for a transformer to find.

Renderers draw a `custom` node they don't know as a container of its
children; a `custom` node with the attribute `inline: true` is inline
(`render-html` uses a `span`, or the element named by `html_tag`, with
`title` as its title).

## Diagnostics

Results and the `diagnostics` notification carry diagnostics: a
`severity`, a `message`, and optionally the `source` file, a `span` in it
and a short `code` (`"broken-link"`). Severities:

| Severity | Meaning |
|---|---|
| `error` | The build fails (after finishing, so every problem is reported). |
| `warning` | Probably a mistake; the build continues. |
| `info`, `hint` | Worth knowing. |

A request that fails outright answers with an error instead (below); the
build stops with that error.

## Errors

```
{"code": <i32>, "message": <string>, "data": <any, optional>}
```

| Code | Meaning |
|---|---|
| -32700 | The message could not be decoded. |
| -32600 | The message is not a valid request. |
| -32601 | No handler for the method. |
| -32602 | The params do not match the method's params type. |
| -32603 | The handler failed. |
| -32800 | The request was cancelled. |

## Cancellation

Either side may cancel a request it sent with the notification
`$/cancel` and params `{"id": <id>}`. The receiver stops work on it if it can
and answers the request with error -32800. A response may still arrive if the
work finished first; the canceller ignores it.

## Caching

A plugin whose results depend only on the request, the site settings and
its name, version and options may say so with `cache` in its capabilities:
a string covering any other input it reads (for example a digest of a
template file), or `""` if there is none. The host may then reuse an
earlier result for the same inputs instead of sending the request. A plugin
without `cache` is never cached. Diagnostics in a cached result are
reported again; notifications sent while producing it are not.

Results of `parse`, `transform/collect`, `transform/apply` and
`transform/collected` may list `dependencies`: files they depend on
besides the request (included files, the source code of generated pages),
as absolute paths or relative to the site directory. A cached result is
reused only while each is unchanged (one that was missing must still be
missing), and `lsdocs serve` rebuilds when one changes, even outside the
site directory.

## Example session

A parser plugin's session, with MessagePack values written as JSON (the
`initialize` params are shortened):

```
host -> [0, 0, "initialize", [{"protocol_version": "1.0-rc.0",
          "host": {"name": "lsdocs", "version": "0.1.0"},
          "site": {"root": "/home/me/site", "files": ["index.txt"]},
          "options": null}]]
plugin -> [1, 0, null, {"protocol_version": "1.0-rc.0",
          "plugin": {"name": "parser-txt", "version": "1.0.0"},
          "capabilities": {"parse": {"extensions": ["txt"]}, "cache": ""}}]
host -> [2, "initialized", []]
host -> [0, 1, "parse", [{"source": "index.txt", "text": "Hello.\n"}]]
plugin -> [1, 1, null, {"document": {"source": "index.txt", "children": [
          {"type": "paragraph", "span": {"start": 0, "end": 6},
           "children": [{"type": "text", "value": "Hello."}]}]}}]
host -> [0, 2, "shutdown", []]
plugin -> [1, 2, null, null]
host -> [2, "exit", []]
```

The notification `initialized` is the 3-element array
`[2, "initialized", []]`, which is 15 bytes of MessagePack:

```
93 02 ab 69 6e 69 74 69 61 6c 69 7a 65 64 90
```

Framed, it is preceded by its length:

```
00 00 00 0f  93 02 ab 69 6e 69 74 69 61 6c 69 7a 65 64 90
```
