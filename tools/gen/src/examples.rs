//! Example values for the fixtures. They should exercise every field shape:
//! present and absent optionals, nested nodes, binary data and free-form
//! values.

use std::collections::BTreeMap;

use lsdocs_doctree::{
    Align, BlockQuote, CodeBlock, Custom, Document, Emphasis, Heading, Image, InlineCode,
    LineBreak, Link, List, ListItem, Paragraph, Raw, Section, Span, Strong, Table, TableCell,
    TableRow, ThematicBreak, text,
};
use lsdocs_protocol::{
    CancelParams, Capabilities, Diagnostic, DiagnosticsParams, InitializeParams, InitializeResult,
    LogLevel, LogParams, PROTOCOL_VERSION, ParseCapability, ParseParams, ParseResult, ProgramInfo,
    ProgressParams, RenderCapability, RenderParams, RenderResult, Severity, SiteInfo,
    TransformApplyParams, TransformApplyResult, TransformCapability, TransformCollectParams,
    TransformCollectResult,
};
use serde_json::json;

/// A document using every node kind.
pub fn document() -> Document {
    let mut doc = Document::new("guide/getting-started.md");
    doc.metadata
        .insert("title".into(), json!("Getting started"));
    doc.metadata
        .insert("tags".into(), json!(["intro", "setup"]));
    doc.metadata.insert("weight".into(), json!(10));

    let cell = |s: &str| {
        TableCell {
            children: vec![text(s)],
            ..TableCell::default()
        }
        .into()
    };

    doc.children = vec![
        Section {
            span: Some(Span::new(0, 412)),
            id: Some("getting-started".into()),
            children: vec![
                Heading {
                    span: Some(Span::new(0, 17)),
                    level: 1,
                    id: Some("getting-started".into()),
                    children: vec![text("Getting started")],
                }
                .into(),
                Paragraph {
                    span: Some(Span::new(19, 98)),
                    children: vec![
                        text("Install "),
                        InlineCode {
                            value: "lsdocs".into(),
                            ..InlineCode::default()
                        }
                        .into(),
                        text(", then read the "),
                        Link {
                            url: "../reference/config.md".into(),
                            title: Some("Configuration reference".into()),
                            children: vec![
                                Emphasis {
                                    children: vec![text("config")],
                                    ..Emphasis::default()
                                }
                                .into(),
                            ],
                            ..Link::default()
                        }
                        .into(),
                        text(" docs."),
                        LineBreak::default().into(),
                        Strong {
                            children: vec![text("Note:")],
                            ..Strong::default()
                        }
                        .into(),
                        text(" Unicode is fine — ✓."),
                    ],
                }
                .into(),
                List {
                    ordered: true,
                    start: Some(3),
                    children: vec![
                        ListItem {
                            checked: Some(true),
                            children: vec![text("Install")],
                            ..ListItem::default()
                        }
                        .into(),
                        ListItem {
                            children: vec![text("Configure")],
                            ..ListItem::default()
                        }
                        .into(),
                    ],
                    ..List::default()
                }
                .into(),
                BlockQuote {
                    children: vec![
                        Paragraph {
                            children: vec![text("Quoted.")],
                            ..Paragraph::default()
                        }
                        .into(),
                    ],
                    ..BlockQuote::default()
                }
                .into(),
                CodeBlock {
                    language: Some("toml".into()),
                    info: Some("title=\"lsdocs.toml\"".into()),
                    value: "[site]\ntitle = \"Docs\"\n".into(),
                    ..CodeBlock::default()
                }
                .into(),
                Table {
                    align: vec![Align::Left, Align::None, Align::Right],
                    children: vec![
                        TableRow {
                            header: true,
                            children: vec![cell("Key"), cell("Type"), cell("Default")],
                            ..TableRow::default()
                        }
                        .into(),
                        TableRow {
                            children: vec![cell("title"), cell("string"), cell("")],
                            ..TableRow::default()
                        }
                        .into(),
                    ],
                    ..Table::default()
                }
                .into(),
                ThematicBreak::default().into(),
                Image {
                    url: "img/arch.svg".into(),
                    alt: "Architecture".into(),
                    ..Image::default()
                }
                .into(),
                Raw {
                    format: "html".into(),
                    value: "<details>".into(),
                    ..Raw::default()
                }
                .into(),
                Custom {
                    kind: "admonition".into(),
                    attrs: BTreeMap::from([
                        ("style".into(), json!("warning")),
                        ("collapsible".into(), json!(false)),
                    ]),
                    children: vec![text("Back up first.")],
                    ..Custom::default()
                }
                .into(),
            ],
        }
        .into(),
    ];
    doc
}

/// One example value per message part.
pub struct Messages {
    pub initialize_params: InitializeParams,
    pub initialize_result: InitializeResult,
    pub parse_params: ParseParams,
    pub parse_result: ParseResult,
    pub collect_params: TransformCollectParams,
    pub collect_result: TransformCollectResult,
    pub apply_params: TransformApplyParams,
    pub apply_result: TransformApplyResult,
    pub render_params: RenderParams,
    pub render_result: RenderResult,
    pub cancel: CancelParams,
    pub log: LogParams,
    pub progress: ProgressParams,
    pub diagnostics: DiagnosticsParams,
}

impl Messages {
    pub fn new() -> Self {
        let doc = document();
        let warning = Diagnostic {
            severity: Severity::Warning,
            message: "link target `../reference/config.md` not found".into(),
            source: Some(doc.source.clone()),
            span: Some(Span::new(60, 97)),
            code: Some("broken-link".into()),
        };
        Self {
            initialize_params: InitializeParams {
                protocol_version: PROTOCOL_VERSION,
                host: ProgramInfo {
                    name: "lsdocs".into(),
                    version: "0.1.0".into(),
                },
                site: SiteInfo {
                    title: Some("My Docs".into()),
                    base_url: Some("/docs/".into()),
                    root: Some("/home/me/my-docs".into()),
                    files: vec!["guide.md".into(), "img/logo.svg".into(), "index.md".into()],
                },
                options: json!({"depth": 3}),
            },
            initialize_result: InitializeResult {
                protocol_version: PROTOCOL_VERSION,
                plugin: ProgramInfo {
                    name: "transform-toc".into(),
                    version: "0.1.0".into(),
                },
                capabilities: Capabilities {
                    parse: Some(ParseCapability {
                        extensions: vec!["md".into(), "markdown".into()],
                    }),
                    transform: Some(TransformCapability {
                        nodes: vec!["heading".into(), "custom:admonition".into()],
                        collect: true,
                    }),
                    render: Some(RenderCapability {
                        formats: vec!["html".into()],
                    }),
                    max_concurrency: Some(1),
                },
            },
            parse_params: ParseParams {
                source: doc.source.clone(),
                text: "# Getting started\n\nInstall `lsdocs`, ...\n".into(),
            },
            parse_result: ParseResult {
                document: doc.clone(),
                diagnostics: vec![],
            },
            collect_params: TransformCollectParams {
                document: doc.clone(),
            },
            collect_result: TransformCollectResult {
                data: json!({"headings": [{"id": "getting-started", "level": 1}]}),
                diagnostics: vec![],
            },
            apply_params: TransformApplyParams {
                document: doc.clone(),
                collected: json!({
                    "guide/getting-started.md": {"headings": [{"id": "getting-started", "level": 1}]},
                    "index.md": {"headings": []},
                }),
            },
            apply_result: TransformApplyResult {
                document: doc.clone(),
                diagnostics: vec![warning.clone()],
            },
            render_params: RenderParams {
                document: doc,
                format: "html".into(),
            },
            render_result: RenderResult {
                output: "<h1 id=\"getting-started\">Getting started</h1>\n"
                    .as_bytes()
                    .to_vec(),
                extension: "html".into(),
                diagnostics: vec![],
            },
            cancel: CancelParams { id: 42 },
            log: LogParams {
                level: LogLevel::Debug,
                message: "building toc".into(),
            },
            progress: ProgressParams {
                message: "rendering".into(),
                percentage: Some(50),
            },
            diagnostics: DiagnosticsParams {
                diagnostics: vec![
                    warning,
                    Diagnostic {
                        severity: Severity::Error,
                        message: "plugin crashed".into(),
                        ..Diagnostic::default()
                    },
                ],
            },
        }
    }
}
