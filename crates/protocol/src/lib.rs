//! Message and capability types for the lsdocs plugin protocol.
//!
//! The host and plugins exchange requests and notifications. Each one is a
//! marker type implementing [`Request`] or [`Notification`], which ties the
//! method name to its parameter and result types; see [`request`] and
//! [`notification`]. How messages are framed and sent is up to the `rpc`
//! crate; this crate only defines what is inside them.
//!
//! ## Lifecycle
//!
//! 1. Host -> plugin: [`request::Initialize`]; the plugin answers with its
//!    [`Capabilities`].
//! 2. Host -> plugin: [`notification::Initialized`].
//! 3. Phase requests, as the plugin's capabilities allow:
//!    [`request::Parse`], [`request::TransformCollect`],
//!    [`request::TransformApply`], [`request::Render`], then once
//!    [`request::RenderSite`].
//! 4. Host -> plugin: [`request::Shutdown`], then [`notification::Exit`].
//!
//! Plugins may send [`notification::Log`], [`notification::Progress`] and
//! [`notification::Diagnostics`] at any time after initialization.
//!
//! ## Encoding
//!
//! Messages are MessagePack. Use [`msgpack::encode`] and [`msgpack::decode`],
//! which write structs as maps (never positional arrays) so that fields can be
//! added without breaking plugins in other languages. Field names are
//! `snake_case`; optional fields are omitted when empty; unknown fields are
//! ignored.

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use lsdocs_doctree as doctree;
use lsdocs_doctree::{Document, Metadata, Span, Value};

/// Protocol version spoken by this crate. Version 0 is unstable: any change
/// may break compatibility until version 1 is frozen.
pub const PROTOCOL_VERSION: u32 = 0;

/// A request: a message that expects a response.
pub trait Request {
    /// Method name on the wire.
    const METHOD: &'static str;
    /// Parameters sent with the request.
    type Params: Serialize + DeserializeOwned;
    /// Value returned on success.
    type Result: Serialize + DeserializeOwned;
}

/// A notification: a message with no response.
pub trait Notification {
    /// Method name on the wire.
    const METHOD: &'static str;
    /// Parameters sent with the notification.
    type Params: Serialize + DeserializeOwned;
}

/// Requests, as marker types.
pub mod request {
    use super::*;

    macro_rules! requests {
        ($( $(#[$m:meta])* $name:ident = $method:literal, $params:ty => $result:ty; )*) => {$(
            $(#[$m])*
            #[derive(Debug)]
            pub enum $name {}
            impl Request for $name {
                const METHOD: &'static str = $method;
                type Params = $params;
                type Result = $result;
            }
        )*};
    }

    requests! {
        /// Host -> plugin. Starts the session and exchanges capabilities.
        Initialize = "initialize", InitializeParams => InitializeResult;
        /// Host -> plugin. Asks the plugin to finish outstanding work; it
        /// should then expect [`notification::Exit`](super::notification::Exit).
        Shutdown = "shutdown", () => ();
        /// Host -> plugin. Parses one source file into a document.
        Parse = "parse", ParseParams => ParseResult;
        /// Host -> plugin. First transform pass: gathers data from one
        /// document, e.g. its headings for a site-wide index.
        TransformCollect = "transform/collect", TransformCollectParams => TransformCollectResult;
        /// Host -> plugin. Second transform pass: rewrites one document, with
        /// the data collected from all documents.
        TransformApply = "transform/apply", TransformApplyParams => TransformApplyResult;
        /// Host -> plugin. Renders one document to an output format.
        Render = "render", RenderParams => RenderResult;
        /// Host -> plugin. Sent once after every document is rendered, if
        /// the renderer asked for it ([`RenderCapability::site`]): returns
        /// site-wide files such as a sitemap.
        RenderSite = "render/site", RenderSiteParams => RenderSiteResult;
    }
}

/// Notifications, as marker types.
pub mod notification {
    use super::*;

    macro_rules! notifications {
        ($( $(#[$m:meta])* $name:ident = $method:literal, $params:ty; )*) => {$(
            $(#[$m])*
            #[derive(Debug)]
            pub enum $name {}
            impl Notification for $name {
                const METHOD: &'static str = $method;
                type Params = $params;
            }
        )*};
    }

    notifications! {
        /// Host -> plugin. Sent after the initialize response; phase requests
        /// may follow.
        Initialized = "initialized", ();
        /// Host -> plugin. The plugin should exit now.
        Exit = "exit", ();
        /// Either direction. Cancels an in-flight request.
        Cancel = "$/cancel", CancelParams;
        /// Plugin -> host. A log line.
        Log = "log", LogParams;
        /// Plugin -> host. Progress on long-running work.
        Progress = "progress", ProgressParams;
        /// Plugin -> host. Diagnostics not tied to a particular response.
        Diagnostics = "diagnostics", DiagnosticsParams;
    }
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

/// Name and version of a program taking part in the protocol.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProgramInfo {
    /// Program name.
    pub name: String,
    /// Program version.
    pub version: String,
}

/// Site-wide settings plugins may need, e.g. for building links.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SiteInfo {
    /// Site title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// URL or path prefix the site is served from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Absolute path of the site's directory (where `lsdocs.toml` is).
    /// Resolve relative paths from plugin options against this, not the
    /// process's working directory: builtin plugins run inside the host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Every file under the source directory (documents and static assets
    /// alike), relative to it with `/` separators, sorted. Lets transformers
    /// check links to files that are not documents, such as images.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// Parameters of [`request::Initialize`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InitializeParams {
    /// Protocol version the host speaks.
    pub protocol_version: u32,
    /// The host.
    pub host: ProgramInfo,
    /// The site being built.
    #[serde(default)]
    pub site: SiteInfo,
    /// The plugin's `options` from the site config, or null.
    #[serde(default)]
    pub options: Value,
}

/// Result of [`request::Initialize`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InitializeResult {
    /// Protocol version the plugin speaks. The host refuses a mismatch.
    pub protocol_version: u32,
    /// The plugin.
    pub plugin: ProgramInfo,
    /// What the plugin can do.
    pub capabilities: Capabilities,
}

/// What a plugin can do. A phase the plugin does not support is omitted.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Capabilities {
    /// Present if the plugin handles [`request::Parse`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse: Option<ParseCapability>,
    /// Present if the plugin handles the transform requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<TransformCapability>,
    /// Present if the plugin handles [`request::Render`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<RenderCapability>,
    /// Most requests the plugin can work on at once; omitted means no
    /// limit. Python plugins report 1 because of the GIL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<u32>,
}

/// Parsing capability.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseCapability {
    /// File extensions handled, without the dot, e.g. `["md", "markdown"]`.
    pub extensions: Vec<String>,
}

/// Transform capability.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TransformCapability {
    /// Node kinds the transformer looks at (see `Node::kind` in the doctree
    /// crate, e.g. `"heading"` or `"custom:mermaid"`). Empty means the whole
    /// document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    /// Whether the plugin handles [`request::TransformCollect`]. If false,
    /// only [`request::TransformApply`] is sent.
    #[serde(default)]
    pub collect: bool,
}

/// Rendering capability.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderCapability {
    /// Output formats produced, e.g. `["html"]`.
    pub formats: Vec<String>,
    /// Whether the plugin handles [`request::RenderSite`].
    #[serde(default, skip_serializing_if = "is_false")]
    pub site: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

// ---------------------------------------------------------------------------
// Phases
// ---------------------------------------------------------------------------

/// Parameters of [`request::Parse`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseParams {
    /// Path of the file relative to the source directory, with `/`
    /// separators. Becomes the document's `source`.
    pub source: String,
    /// The file's text.
    pub text: String,
}

/// Result of [`request::Parse`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseResult {
    /// The parsed document.
    pub document: Document,
    /// Problems found while parsing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Parameters of [`request::TransformCollect`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TransformCollectParams {
    /// The document to collect from.
    pub document: Document,
}

/// Result of [`request::TransformCollect`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TransformCollectResult {
    /// Data gathered from the document, in any shape the plugin likes.
    #[serde(default)]
    pub data: Value,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Parameters of [`request::TransformApply`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TransformApplyParams {
    /// The document to transform.
    pub document: Document,
    /// What this plugin collected from every document: a map from each
    /// document's `source` to its collected `data`. Null if the plugin does
    /// not collect.
    #[serde(default)]
    pub collected: Value,
}

/// Result of [`request::TransformApply`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TransformApplyResult {
    /// The transformed document.
    pub document: Document,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Parameters of [`request::Render`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderParams {
    /// The document to render.
    pub document: Document,
    /// Output format, one of the plugin's [`RenderCapability::formats`].
    pub format: String,
}

/// Result of [`request::Render`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderResult {
    /// The rendered output. Sent as MessagePack `bin`.
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<u8>"))]
    pub output: Vec<u8>,
    /// File extension for the output, without the dot, e.g. `"html"`.
    pub extension: String,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Parameters of [`request::RenderSite`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderSiteParams {
    /// Output format the pages were rendered to.
    pub format: String,
    /// Every page rendered, sorted by source path.
    #[serde(default)]
    pub pages: Vec<RenderedPage>,
}

/// A page written by the render phase.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderedPage {
    /// The document's source path.
    pub source: String,
    /// Where the page was written, relative to the output directory, with
    /// `/` separators, e.g. `guide/intro.html`.
    pub path: String,
    /// The document's metadata after the transform phase.
    #[serde(default, skip_serializing_if = "Metadata::is_empty")]
    pub metadata: Metadata,
}

/// Result of [`request::RenderSite`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderSiteResult {
    /// Extra files to write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<OutputFile>,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// A file for the host to write.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OutputFile {
    /// Path relative to the output directory, with `/` separators. It may
    /// not leave the output directory or replace a rendered page.
    pub path: String,
    /// The file's content. Sent as MessagePack `bin`.
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<u8>"))]
    pub output: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Diagnostics and notifications
// ---------------------------------------------------------------------------

/// A problem to report to the user, modeled on LSP diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Diagnostic {
    /// How serious the problem is.
    pub severity: Severity,
    /// What is wrong.
    pub message: String,
    /// Source file the problem is in, relative to the source directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Where in `source` the problem is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
    /// A short identifier for the kind of problem, e.g. `"broken-link"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Severity of a [`Diagnostic`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The build fails.
    #[default]
    Error,
    /// Probably a mistake, but the build continues.
    Warning,
    /// Worth knowing.
    Info,
    /// A suggestion.
    Hint,
}

/// Parameters of [`notification::Cancel`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CancelParams {
    /// Id of the request to cancel.
    pub id: u32,
}

/// Parameters of [`notification::Log`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LogParams {
    /// Log level.
    pub level: LogLevel,
    /// The message.
    pub message: String,
}

/// Level of a [`LogParams`] message.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    /// Errors.
    Error,
    /// Warnings.
    Warn,
    /// General information.
    #[default]
    Info,
    /// Debugging detail.
    Debug,
    /// Very verbose detail.
    Trace,
}

/// Parameters of [`notification::Progress`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProgressParams {
    /// What is being worked on.
    pub message: String,
    /// Percent complete, 0-100, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage: Option<u8>,
}

/// Parameters of [`notification::Diagnostics`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiagnosticsParams {
    /// The diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// MessagePack encoding following the protocol's rules.
pub mod msgpack {
    use serde::{Serialize, de::DeserializeOwned};

    pub use rmp_serde::decode::Error as DecodeError;
    pub use rmp_serde::encode::Error as EncodeError;

    /// Encodes a value. Structs become maps keyed by field name, never
    /// positional arrays.
    pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, EncodeError> {
        rmp_serde::to_vec_named(value)
    }

    /// Decodes a value.
    pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
        rmp_serde::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsdocs_doctree::{Heading, text};
    use std::fmt::Debug;

    fn round_trip<T>(value: &T) -> Vec<u8>
    where
        T: Serialize + DeserializeOwned + PartialEq + Debug,
    {
        let bytes = msgpack::encode(value).unwrap();
        assert_eq!(&msgpack::decode::<T>(&bytes).unwrap(), value);
        bytes
    }

    /// Whether `bytes` starts with a MessagePack map header.
    fn is_map(bytes: &[u8]) -> bool {
        matches!(bytes[0], 0x80..=0x8f | 0xde | 0xdf)
    }

    #[test]
    fn structs_encode_as_maps() {
        let bytes = round_trip(&ProgramInfo {
            name: "lsdocs".into(),
            version: "0.1.0".into(),
        });
        assert!(is_map(&bytes), "{bytes:x?}");
    }

    #[test]
    fn unit_enums_encode_as_strings() {
        let bytes = round_trip(&Severity::Warning);
        assert_eq!(bytes, b"\xa7warning");
    }

    #[test]
    fn render_output_is_binary() {
        let bytes = round_trip(&RenderResult {
            output: b"<p>hi</p>".to_vec(),
            extension: "html".into(),
            diagnostics: vec![],
        });
        // `bin 8` header followed by the length, then the raw bytes.
        let needle = b"\xc4\x09<p>hi</p>";
        assert!(
            bytes.windows(needle.len()).any(|w| w == needle),
            "{bytes:x?}"
        );
    }

    #[test]
    fn documents_round_trip_through_msgpack() {
        let mut document = Document::new("index.md");
        document.children.push(
            Heading {
                level: 2,
                children: vec![text("Hello")],
                ..Heading::default()
            }
            .into(),
        );
        round_trip(&TransformApplyParams {
            document,
            collected: serde_json::json!({"index.md": ["Hello"]}),
        });
    }

    #[test]
    fn method_names() {
        assert_eq!(request::TransformCollect::METHOD, "transform/collect");
        assert_eq!(notification::Cancel::METHOD, "$/cancel");
        assert_eq!(request::RenderSite::METHOD, "render/site");
    }
}
