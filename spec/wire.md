# lsdocs wire protocol

Status: **draft, protocol version 0** (unstable until version 1 is frozen).

This document describes the bytes exchanged between the lsdocs host and a
plugin. It is enough to write a plugin in any language without the official
bindings. The message *contents* (params and results) are described by the
JSON Schemas in [`schema/protocol/`](../schema/protocol), and
[`fixtures/`](../fixtures) has an encoded example of each.

## Transports

A plugin talks to the host over one of:

| Transport | How it is set up |
|---|---|
| `stdio` (default) | The host launches the plugin and uses its stdin and stdout. |
| `pipe` | The host creates a named pipe (Windows) or unix socket and launches the plugin with its name in the `LSDOCS_PIPE` environment variable. The plugin connects to it. |

First-party plugins may also run in-process, which has no wire format.

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
handshake. Responses may arrive in any order.

### Encoding rules

These keep messages readable from any language and let fields be added
without breaking older peers:

- Structs are **maps keyed by field name**, never positional arrays.
- Field names are `snake_case`.
- Optional fields are omitted when absent. Receivers must treat a missing
  optional field as absent and **ignore unknown fields**.
- Enums without data are strings (`"warning"`). Document nodes are maps with
  a `"type"` field naming the node kind.

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
- Binary data (e.g. render output) is MessagePack `bin`.
- MessagePack extension types are not used.

### Errors

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

## Lifecycle

1. Host -> plugin request `initialize`. The plugin answers with its protocol
   version, name and capabilities. The host ends the connection if the
   protocol versions differ.
2. Host -> plugin notification `initialized`.
3. Phase requests the plugin's capabilities allow: `parse`,
   `transform/collect`, then once `transform/collected` (everything the
   plugin collected; the `transform/apply` requests that follow leave
   `collected` null), `transform/apply`, `render`, and, once after every
   document is rendered, `render/site` (if the render capability has
   `site: true`), whose result lists extra files such as a sitemap. The
   plugin may send `log`, `progress` and `diagnostics` notifications at any
   time.
4. Host -> plugin request `shutdown`, then notification `exit`. The plugin
   exits; closing the stream also ends the session.

## Caching

A plugin whose results depend only on the request, the site settings and
its name, version and options may say so with `cache` in its capabilities:
a string covering any other input it reads (for example a digest of a
template file), or `""` if there is none. The host may then reuse an
earlier result for the same inputs instead of sending the request. A plugin
without `cache` is never cached. Diagnostics in a cached result are
reported again; notifications sent while producing it are not.

## Example

The notification `initialized` (no params) is the 3-element array
`[2, "initialized", []]`, which is 15 bytes of MessagePack:

```
93 02 ab 69 6e 69 74 69 61 6c 69 7a 65 64 90
```

Framed, it is preceded by its length:

```
00 00 00 0f  93 02 ab 69 6e 69 74 69 61 6c 69 7a 65 64 90
```
