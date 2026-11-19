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
//!    [`request::Parse`], [`request::ParseTransformEach`], then once
//!    [`request::ParseTransformAll`], [`request::RenderTransformEach`],
//!    [`request::Render`], then once [`request::RenderTransformAll`].
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

/// Protocol version spoken by this crate: `MAJOR.MINOR`, with `-rc.N` for
/// a release candidate. See [`compatible`] and "Versions" in
/// `spec/wire.md`.
pub const PROTOCOL_VERSION: &str = "1.0-rc.0";

/// Whether peers speaking protocol versions `a` and `b` can talk: the same
/// major version, and, if either is a release candidate, the very same
/// version. Minor versions only add things older peers can ignore.
pub fn compatible(a: &str, b: &str) -> bool {
    let major = |v: &str| v.split(['.', '-']).next().map(str::to_owned);
    let prerelease = |v: &str| v.contains('-');
    if prerelease(a) || prerelease(b) {
        return a == b;
    }
    major(a).is_some_and(|m| !m.is_empty() && major(b) == Some(m))
}

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
        ParseTransformEach = "parse/transform/each", ParseTransformEachParams => ParseTransformEachResult;
        /// Host -> plugin. Sent once between the two transform passes, to a
        /// plugin that collects: everything collected from all documents.
        /// The `render/transform/each` requests that follow omit it. The plugin
        /// may answer with documents it generates from that data (an index
        /// page, say), which join the site.
        ParseTransformAll = "parse/transform/all", ParseTransformAllParams => ParseTransformAllResult;
        /// Host -> plugin. Second transform pass: rewrites one document, with
        /// the data collected from all documents.
        RenderTransformEach = "render/transform/each", RenderTransformEachParams => RenderTransformEachResult;
        /// Host -> plugin. Renders one document to an output format.
        Render = "render", RenderParams => RenderResult;
        /// Plugin -> host. Parses a piece of text with the parser that
        /// takes files of an extension, e.g. a doc comment written in
        /// Markdown, while the plugin works on a request. The result is not
        /// normalized.
        HostParse = "host/parse", HostParseParams => ParseResult;
        /// Host -> plugin. Sent once after every document is rendered, if
        /// the renderer asked for it ([`RenderCapability::site`]): returns
        /// site-wide files such as a sitemap.
        RenderTransformAll = "render/transform/all", RenderTransformAllParams => RenderTransformAllResult;
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
    /// Resolve relative paths from plugin settings against this, not the
    /// process's working directory: builtin plugins run inside the host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Absolute path of the source directory, which document `source`
    /// paths are relative to (e.g. for parsers that read included files).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<String>,
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
    /// Protocol version the host speaks, e.g. `"1.0"`.
    pub protocol_version: String,
    /// The host.
    pub host: ProgramInfo,
    /// The site being built.
    #[serde(default)]
    pub site: SiteInfo,
    /// Where the plugin's settings are.
    #[serde(default)]
    pub config: ConfigInfo,
}

/// Where a plugin's settings are: the site's `.config/` directory, whose
/// layout is the plugin's business (the recommended one is
/// `<vendor>/<name>/`), and the plugin's name in the site config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigInfo {
    /// Absolute path of `.config/` (next to `lsdocs.toml`), whether or
    /// not it exists.
    #[serde(default)]
    pub dir: String,
    /// The plugin's name in the site config (`name = "..."`), so that two
    /// instances of a plugin can keep separate settings.
    #[serde(default)]
    pub name: String,
}

/// Result of [`request::Initialize`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InitializeResult {
    /// Protocol version the plugin speaks. The host ends the session if it
    /// is not [`compatible`] with its own.
    pub protocol_version: String,
    /// The plugin.
    pub plugin: ProgramInfo,
    /// What the plugin can do.
    pub capabilities: Capabilities,
    /// The files the plugin read its settings from, relative to
    /// `.config/` (or absolute), including ones it looked for and did not
    /// find. The host digests them into the plugin's cache keys, so a
    /// change to one (or creating one) is a change of settings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_files: Vec<String>,
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
    /// Present if the plugin's results may be cached: they depend only on
    /// the request, the site settings, the plugin's name, version and
    /// options, and this key. A plugin that reads anything else (say, a
    /// template file its options name) puts a digest of it in the key, so
    /// the cache is invalidated when that changes. Omitted means never
    /// cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    /// Directives and roles the plugin handles. The host passes every
    /// plugin's syntax to parsers ([`ParseParams::syntax`]), which turn it
    /// into `custom` nodes for the plugin to transform.
    #[serde(default, skip_serializing_if = "Syntax::is_empty")]
    pub syntax: Syntax,
}

/// Syntax extensions: directives and roles that plugins give meaning to.
///
/// Parsers read them with their format's syntax (in reStructuredText,
/// `.. name:: argument` and `` :name:`text` ``; in Markdown, MyST's
/// ```` ```{name} argument ```` fences and `` {name}`text` ``) and make
/// format-neutral nodes of them; see "Syntax extensions" in
/// `spec/wire.md` for the shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Syntax {
    /// Block-level directives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directives: Vec<DirectiveSyntax>,
    /// Inline roles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<RoleSyntax>,
}

impl Syntax {
    /// Whether there are neither directives nor roles.
    pub fn is_empty(&self) -> bool {
        self.directives.is_empty() && self.roles.is_empty()
    }

    /// Adds `other`'s directives and roles, keeping the first declaration
    /// of a name.
    pub fn merge(&mut self, other: &Syntax) {
        for d in &other.directives {
            if !self.directives.iter().any(|x| x.name == d.name) {
                self.directives.push(d.clone());
            }
        }
        for r in &other.roles {
            if !self.roles.iter().any(|x| x.name == r.name) {
                self.roles.push(r.clone());
            }
        }
    }

    /// The directive called `name`, if declared.
    pub fn directive(&self, name: &str) -> Option<&DirectiveSyntax> {
        self.directives.iter().find(|d| d.name == name)
    }

    /// Whether a role called `name` is declared.
    pub fn has_role(&self, name: &str) -> bool {
        self.roles.iter().any(|r| r.name == name)
    }
}

/// A directive: what it accepts, so parsers can split its parts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DirectiveSyntax {
    /// The directive's name, in lower case, e.g. `"versionadded"`.
    pub name: String,
    /// How many arguments it requires.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub required_arguments: u32,
    /// How many more arguments it accepts.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub optional_arguments: u32,
    /// Whether the last argument may contain whitespace (it takes the rest
    /// of the argument text).
    #[serde(default, skip_serializing_if = "is_false")]
    pub final_argument_whitespace: bool,
    /// How the argument is read.
    #[serde(default, skip_serializing_if = "ArgumentSyntax::is_text")]
    pub argument: ArgumentSyntax,
    /// The options it accepts, by name.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub options: std::collections::BTreeMap<String, OptionSyntax>,
    /// What its content is.
    #[serde(default, skip_serializing_if = "ContentSyntax::is_none")]
    pub content: ContentSyntax,
}

/// How a directive's argument is read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ArgumentSyntax {
    /// Plain text, kept as written.
    #[default]
    Text,
    /// Inline markup, parsed as well as kept as text.
    Inline,
}

impl ArgumentSyntax {
    fn is_text(&self) -> bool {
        *self == Self::Text
    }
}

/// What a directive option takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum OptionSyntax {
    /// No value.
    Flag,
    /// A value, which may be empty.
    #[default]
    Value,
    /// A value that must not be empty.
    Required,
}

/// What a directive's content is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ContentSyntax {
    /// The directive takes no content.
    #[default]
    None,
    /// Block markup, parsed into the node's children.
    Blocks,
    /// Text kept as written (code, a list of document names, ...).
    Text,
}

impl ContentSyntax {
    fn is_none(&self) -> bool {
        *self == Self::None
    }
}

/// A role: inline text with a name, e.g. `` :kbd:`Ctrl+C` ``.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RoleSyntax {
    /// The role's name, in lower case, e.g. `"kbd"`.
    pub name: String,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
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
    /// Whether the plugin handles [`request::ParseTransformEach`]. If false,
    /// only [`request::RenderTransformEach`] is sent.
    #[serde(default)]
    pub each: bool,
}

/// Rendering capability.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderCapability {
    /// Output formats produced, e.g. `["html"]`.
    pub formats: Vec<String>,
    /// Whether the plugin handles [`request::RenderTransformAll`].
    #[serde(default, skip_serializing_if = "is_false")]
    pub all: bool,
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
    /// The directives and roles plugins declared ([`Capabilities::syntax`]),
    /// for parsers that support syntax extensions.
    #[serde(default, skip_serializing_if = "Syntax::is_empty")]
    pub syntax: Syntax,
}

/// Parameters of [`request::HostParse`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HostParseParams {
    /// The extension of the files whose parser should parse the text,
    /// without the dot, e.g. `"md"` or `"rst"`.
    pub extension: String,
    /// The source path to give the parsed document (and its diagnostics),
    /// usually the document the text will be part of.
    pub source: String,
    /// The text.
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
    /// Files the result depends on besides the request, such as included
    /// files: absolute paths, or paths relative to the site directory
    /// (`site.root`), with `/` separators. A cached result is only reused
    /// while they are unchanged (a file that did not exist must still not
    /// exist), and `lsdocs serve` rebuilds when they change. List files
    /// looked for but missing too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

/// Parameters of [`request::ParseTransformEach`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseTransformEachParams {
    /// The document to collect from.
    pub document: Document,
}

/// Result of [`request::ParseTransformEach`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseTransformEachResult {
    /// Data gathered from the document, in any shape the plugin likes.
    #[serde(default)]
    pub data: Value,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    /// Files the result depends on besides the request, such as included
    /// files: absolute paths, or paths relative to the site directory
    /// (`site.root`), with `/` separators. A cached result is only reused
    /// while they are unchanged (a file that did not exist must still not
    /// exist), and `lsdocs serve` rebuilds when they change. List files
    /// looked for but missing too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

/// Parameters of [`request::ParseTransformAll`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseTransformAllParams {
    /// What this plugin collected from every document: a map from each
    /// document's `source` to its collected `data`.
    #[serde(default)]
    pub collected: Value,
}

/// Result of [`request::ParseTransformAll`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseTransformAllResult {
    /// Documents the plugin generates, with no source file: each one's
    /// `source` is a path no source file has (it names the output, as a
    /// source path would). The host normalizes them, and they go through
    /// this plugin's apply pass, the transformers after it and rendering
    /// like any other document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<Document>,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    /// Files the result depends on besides the request, such as included
    /// files: absolute paths, or paths relative to the site directory
    /// (`site.root`), with `/` separators. A cached result is only reused
    /// while they are unchanged (a file that did not exist must still not
    /// exist), and `lsdocs serve` rebuilds when they change. List files
    /// looked for but missing too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

/// Parameters of [`request::RenderTransformEach`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderTransformEachParams {
    /// The document to transform.
    pub document: Document,
    /// What this plugin collected from every document: a map from each
    /// document's `source` to its collected `data`. Null if the plugin does
    /// not collect. Over the wire the host leaves it null and sends it once
    /// beforehand with [`request::ParseTransformAll`]; the plugin SDK fills
    /// it back in, so handlers always see it here.
    #[serde(default)]
    pub collected: Value,
}

/// Result of [`request::RenderTransformEach`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderTransformEachResult {
    /// The transformed document.
    pub document: Document,
    /// Problems found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    /// Files the result depends on besides the request, such as included
    /// files: absolute paths, or paths relative to the site directory
    /// (`site.root`), with `/` separators. A cached result is only reused
    /// while they are unchanged (a file that did not exist must still not
    /// exist), and `lsdocs serve` rebuilds when they change. List files
    /// looked for but missing too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
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
    /// Data about the page, in any shape the plugin likes, handed back to
    /// it in [`RenderedPage::data`] at [`request::RenderTransformAll`]
    /// (a search index's entry for the page, say). Cached with the
    /// result, so a page that is not rendered again keeps it. Omitted
    /// when null.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

/// Parameters of [`request::RenderTransformAll`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderTransformAllParams {
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
    /// The `data` of the page's [`RenderResult`]. Omitted when null.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
}

/// Result of [`request::RenderTransformAll`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderTransformAllResult {
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
    #![allow(clippy::unwrap_used)]
    #[test]
    fn versions_are_compatible_within_a_major_version() {
        use super::compatible;
        assert!(compatible("1.0", "1.3"));
        assert!(!compatible("1.0", "2.0"));
        assert!(compatible("1.0-rc.0", "1.0-rc.0"));
        assert!(!compatible("1.0-rc.0", "1.0-rc.1"));
        assert!(!compatible("1.0-rc.0", "1.0"));
        assert!(!compatible("", ""));
    }

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
            ..RenderResult::default()
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
        round_trip(&RenderTransformEachParams {
            document,
            collected: serde_json::json!({"index.md": ["Hello"]}),
        });
    }

    #[test]
    fn method_names() {
        assert_eq!(request::ParseTransformEach::METHOD, "parse/transform/each");
        assert_eq!(request::ParseTransformAll::METHOD, "parse/transform/all");
        assert_eq!(
            request::RenderTransformEach::METHOD,
            "render/transform/each"
        );
        assert_eq!(request::RenderTransformAll::METHOD, "render/transform/all");
        assert_eq!(request::HostParse::METHOD, "host/parse");
        assert_eq!(notification::Cancel::METHOD, "$/cancel");
        let each = serde_json::to_value(TransformCapability {
            each: true,
            ..TransformCapability::default()
        })
        .unwrap();
        assert_eq!(each["each"], true);
        let all = serde_json::to_value(RenderCapability {
            all: true,
            ..RenderCapability::default()
        })
        .unwrap();
        assert_eq!(all["all"], true);
    }
}
