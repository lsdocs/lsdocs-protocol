//! Site configuration and plugin manifest types.
//!
//! [`ConfigFile`] mirrors `lsdocs.toml` as written. [`Config::from_toml`]
//! parses and validates it into a [`Config`], reporting every problem at once.
//! [`Overrides`] then layer environment variables and CLI flags on top:
//!
//! ```
//! # use lsdocs_config::{Config, Overrides};
//! let mut config = Config::from_toml("[site]\ntitle = \"Docs\"\n").unwrap();
//! config.apply(Overrides::from_env([("LSDOCS_OUTPUT_DIR", "public")]));
//! config.apply(Overrides { base_url: Some("/docs/".into()), ..Overrides::default() });
//! assert_eq!(config.output_dir, "public");
//! ```
//!
//! This crate does no I/O; the caller reads the file and passes the
//! environment in.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// An arbitrary structured value.
pub type Value = serde_json::Value;

/// Default for [`Config::source_dir`].
pub const DEFAULT_SOURCE_DIR: &str = "docs";
/// Default for [`Config::output_dir`].
pub const DEFAULT_OUTPUT_DIR: &str = "site";

// ---------------------------------------------------------------------------
// File format
// ---------------------------------------------------------------------------

/// The contents of `lsdocs.toml`, as written. Unknown keys are rejected so
/// typos are caught.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    /// Site-wide settings.
    #[serde(default)]
    pub site: SiteSection,
    /// Build settings.
    #[serde(default)]
    pub build: BuildSection,
    /// Plugins, in pipeline order within each phase.
    #[serde(default)]
    pub plugins: Vec<PluginEntry>,
}

/// The `[site]` table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SiteSection {
    /// Site title.
    pub title: Option<String>,
    /// URL or path prefix the site is served from, e.g. `"/docs/"`.
    pub base_url: Option<String>,
    /// No longer supported, and an error: the theme is chosen in the
    /// HTML renderer's settings (`theme` in render-html's
    /// `.config/lsdocs/<name>/settings.toml`). Kept so the error can say so.
    pub theme: Option<Value>,
}

/// The `[build]` table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BuildSection {
    /// Directory holding source documents. Defaults to `"docs"`.
    pub source_dir: Option<String>,
    /// Directory the site is written to. Defaults to `"site"`.
    pub output_dir: Option<String>,
}

/// One `[[plugins]]` entry. Exactly one of `builtin`, `command` or `runtime`
/// says how to run the plugin.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PluginEntry {
    /// Unique name used in logs and diagnostics.
    pub name: String,
    /// Name of a first-party plugin to run in-process, e.g.
    /// `"parser-myst"`.
    pub builtin: Option<String>,
    /// Program and arguments to launch.
    pub command: Option<Vec<String>>,
    /// Runtime to launch a script plugin with; needs `module` (Python) or
    /// `entry` (Node).
    pub runtime: Option<Runtime>,
    /// Python module to run with `python -m`.
    pub module: Option<String>,
    /// Node.js entry script.
    pub entry: Option<String>,
    /// How to talk to an out-of-process plugin. Defaults to `"stdio"`.
    pub transport: Option<Transport>,
    /// Glob patterns limiting which source files this plugin handles.
    pub files: Option<Vec<String>>,
    /// No longer supported, and an error: a plugin's settings are its own
    /// files under `.config/`, next to `lsdocs.toml`. Kept so the error
    /// can say so.
    pub options: Option<Value>,
}

/// Runtime for script plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    /// Run with `python -m <module>`.
    Python,
    /// Run with `node <entry>`.
    Node,
}

/// Transport for out-of-process plugins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    /// The plugin's stdin and stdout.
    #[default]
    Stdio,
    /// A named pipe (Windows) or unix socket whose path is passed to the
    /// plugin in the `LSDOCS_PIPE` environment variable.
    Pipe,
}

// ---------------------------------------------------------------------------
// Validated configuration
// ---------------------------------------------------------------------------

/// Validated site configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Site title.
    pub title: Option<String>,
    /// URL or path prefix the site is served from.
    pub base_url: Option<String>,
    /// Directory holding source documents, relative to the config file.
    pub source_dir: String,
    /// Directory the site is written to, relative to the config file.
    pub output_dir: String,
    /// Plugins, in the order they were declared.
    pub plugins: Vec<Plugin>,
}

/// A validated plugin entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    /// Unique name.
    pub name: String,
    /// How the plugin is run.
    pub launch: Launch,
    /// Glob patterns limiting which source files this plugin handles; empty
    /// means no limit beyond the plugin's own capabilities.
    pub files: Vec<String>,
}

/// How a plugin is run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// A first-party plugin linked into the host and run in-process.
    Builtin {
        /// First-party plugin name.
        name: String,
    },
    /// A separate process.
    External {
        /// What to launch.
        command: Command,
        /// How to talk to it.
        transport: Transport,
    },
}

/// What to launch for an out-of-process plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// A program and its arguments.
    Program {
        /// Program name or path.
        program: String,
        /// Arguments.
        args: Vec<String>,
    },
    /// `python -m <module>`.
    Python {
        /// Module name.
        module: String,
    },
    /// `node <entry>`.
    Node {
        /// Entry script path.
        entry: String,
    },
}

impl Config {
    /// Parses and validates the contents of `lsdocs.toml`.
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile =
            toml::from_str(source).map_err(|e| ConfigError::Syntax(e.to_string()))?;
        Self::from_file(file)
    }

    /// Validates an already-parsed config file.
    pub fn from_file(file: ConfigFile) -> Result<Self, ConfigError> {
        let mut problems = Vec::new();
        let mut seen = BTreeSet::new();
        let mut plugins = Vec::with_capacity(file.plugins.len());

        if let Some(url) = &file.site.base_url
            && url.trim().is_empty()
        {
            problems.push(Problem::new("site.base_url", "must not be empty"));
        }
        if file.site.theme.is_some() {
            problems.push(Problem::new(
                "site.theme",
                "`theme` moved: the HTML theme is chosen in render-html's settings \
                 (`theme = \"...\"` in `.config/lsdocs/<name>/settings.toml`, `<name>` being \
                 the render-html plugin's name)",
            ));
        }

        for (i, entry) in file.plugins.into_iter().enumerate() {
            let at = format!("plugins[{i}]");
            if entry.name.trim().is_empty() {
                problems.push(Problem::new(&at, "`name` must not be empty"));
            } else if !seen.insert(entry.name.clone()) {
                problems.push(Problem::new(
                    &at,
                    format!("duplicate plugin name `{}`", entry.name),
                ));
            }
            if let Some(plugin) = validate_plugin(entry, &at, &mut problems) {
                plugins.push(plugin);
            }
        }

        if !problems.is_empty() {
            return Err(ConfigError::Invalid(problems));
        }
        Ok(Self {
            title: file.site.title,
            base_url: file.site.base_url,
            source_dir: file
                .build
                .source_dir
                .unwrap_or_else(|| DEFAULT_SOURCE_DIR.into()),
            output_dir: file
                .build
                .output_dir
                .unwrap_or_else(|| DEFAULT_OUTPUT_DIR.into()),
            plugins,
        })
    }

    /// Applies overrides on top of this configuration. Call once per layer,
    /// lowest priority first (environment, then CLI).
    pub fn apply(&mut self, overrides: Overrides) {
        if let Some(v) = overrides.source_dir {
            self.source_dir = v;
        }
        if let Some(v) = overrides.output_dir {
            self.output_dir = v;
        }
        if let Some(v) = overrides.base_url {
            self.base_url = Some(v);
        }
    }
}

fn validate_plugin(entry: PluginEntry, at: &str, problems: &mut Vec<Problem>) -> Option<Plugin> {
    let start = problems.len();
    let launchers = [
        entry.builtin.is_some(),
        entry.command.is_some(),
        entry.runtime.is_some(),
    ]
    .into_iter()
    .filter(|set| *set)
    .count();
    if launchers != 1 {
        problems.push(Problem::new(
            at,
            "set exactly one of `builtin`, `command` or `runtime`",
        ));
    }
    if entry.module.is_some() && entry.runtime != Some(Runtime::Python) {
        problems.push(Problem::new(at, "`module` needs `runtime = \"python\"`"));
    }
    if entry.entry.is_some() && entry.runtime != Some(Runtime::Node) {
        problems.push(Problem::new(at, "`entry` needs `runtime = \"node\"`"));
    }
    if entry.builtin.is_some() && entry.transport.is_some() {
        problems.push(Problem::new(
            at,
            "`transport` does not apply to builtin plugins, which run in-process",
        ));
    }

    let transport = entry.transport.unwrap_or_default();
    let launch = if let Some(name) = entry.builtin {
        Some(Launch::Builtin { name })
    } else if let Some(mut argv) = entry.command {
        if argv.is_empty() || argv[0].trim().is_empty() {
            problems.push(Problem::new(at, "`command` must name a program"));
            None
        } else {
            let program = argv.remove(0);
            Some(Launch::External {
                command: Command::Program {
                    program,
                    args: argv,
                },
                transport,
            })
        }
    } else {
        match entry.runtime {
            Some(Runtime::Python) => match entry.module {
                Some(module) => Some(Launch::External {
                    command: Command::Python { module },
                    transport,
                }),
                None => {
                    problems.push(Problem::new(at, "`runtime = \"python\"` needs `module`"));
                    None
                }
            },
            Some(Runtime::Node) => match entry.entry {
                Some(entry) => Some(Launch::External {
                    command: Command::Node { entry },
                    transport,
                }),
                None => {
                    problems.push(Problem::new(at, "`runtime = \"node\"` needs `entry`"));
                    None
                }
            },
            None => None,
        }
    };

    if entry.options.is_some() {
        problems.push(Problem::new(
            at,
            "`options` moved: a plugin's settings are its own files under `.config/`, \
             next to `lsdocs.toml` (lsdocs' plugins read `.config/lsdocs/<name>/settings.toml`)",
        ));
    }
    if problems.len() > start {
        return None;
    }
    Some(Plugin {
        name: entry.name,
        launch: launch?,
        files: entry.files.unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// Overrides
// ---------------------------------------------------------------------------

/// Settings that can be overridden by environment variables or CLI flags.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    /// Overrides [`Config::source_dir`] (`LSDOCS_SOURCE_DIR`).
    pub source_dir: Option<String>,
    /// Overrides [`Config::output_dir`] (`LSDOCS_OUTPUT_DIR`).
    pub output_dir: Option<String>,
    /// Overrides [`Config::base_url`] (`LSDOCS_BASE_URL`).
    pub base_url: Option<String>,
}

impl Overrides {
    /// Reads overrides from `LSDOCS_*` variables. Pass `std::env::vars()`.
    pub fn from_env<K, V>(vars: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut out = Self::default();
        for (key, value) in vars {
            let slot = match key.as_ref() {
                "LSDOCS_SOURCE_DIR" => &mut out.source_dir,
                "LSDOCS_OUTPUT_DIR" => &mut out.output_dir,
                "LSDOCS_BASE_URL" => &mut out.base_url,
                _ => continue,
            };
            *slot = Some(value.into());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a configuration could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The file is not valid TOML, or does not match the expected shape
    /// (including unknown keys). Holds the parser's message.
    Syntax(String),
    /// The file parsed but has one or more invalid settings.
    Invalid(Vec<Problem>),
}

/// One invalid setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// Where the problem is, e.g. `plugins[2]`.
    pub at: String,
    /// What is wrong.
    pub message: String,
}

impl Problem {
    fn new(at: &str, message: impl Into<String>) -> Self {
        Self {
            at: at.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Syntax(msg) => write!(f, "invalid config file: {msg}"),
            ConfigError::Invalid(problems) => {
                write!(f, "invalid config file:")?;
                for p in problems {
                    write!(f, "\n  {}: {}", p.at, p.message)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;

    const FULL: &str = r#"
[site]
title = "My Docs"
base_url = "/docs/"

[build]
source_dir = "content"

[[plugins]]
name = "markdown"
builtin = "parser-myst"

[[plugins]]
name = "toc"
runtime = "python"
module = "my_toc"
files = ["guide/**"]

[[plugins]]
name = "html"
command = ["render-html", "--pretty"]
transport = "pipe"
"#;

    #[test]
    fn site_theme_is_an_error() {
        let err = Config::from_toml("[site]\ntheme = \"default\"\n")
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "invalid config file:\n  site.theme: `theme` moved: the HTML theme is chosen in \
             render-html's settings (`theme = \"...\"` in `.config/lsdocs/<name>/settings.toml`, \
             `<name>` being the render-html plugin's name)"
        );
    }

    #[test]
    fn options_key_is_an_error() {
        let err = Config::from_toml(
            "[[plugins]]\nname = \"toc\"\nbuiltin = \"transform-toc\"\noptions = { depth = 3 }\n",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("`options` moved"), "{err}");
        assert!(err.contains(".config/"), "{err}");
    }

    #[test]
    fn full_config_validates() {
        let config = Config::from_toml(FULL).unwrap();
        assert_eq!(config.title.as_deref(), Some("My Docs"));
        assert_eq!(config.source_dir, "content");
        assert_eq!(config.output_dir, DEFAULT_OUTPUT_DIR);
        assert_eq!(
            config.plugins[0].launch,
            Launch::Builtin {
                name: "parser-myst".into()
            }
        );
        assert_eq!(
            config.plugins[1].launch,
            Launch::External {
                command: Command::Python {
                    module: "my_toc".into()
                },
                transport: Transport::Stdio,
            }
        );
        assert_eq!(config.plugins[1].files, ["guide/**"]);
        assert_eq!(
            config.plugins[2].launch,
            Launch::External {
                command: Command::Program {
                    program: "render-html".into(),
                    args: vec!["--pretty".into()],
                },
                transport: Transport::Pipe,
            }
        );
    }

    #[test]
    fn empty_file_uses_defaults() {
        let config = Config::from_toml("").unwrap();
        assert_eq!(config.source_dir, DEFAULT_SOURCE_DIR);
        assert_eq!(config.output_dir, DEFAULT_OUTPUT_DIR);
        assert!(config.plugins.is_empty());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = Config::from_toml("[site]\ntitel = \"typo\"\n").unwrap_err();
        assert!(
            matches!(&err, ConfigError::Syntax(msg) if msg.contains("titel")),
            "{err}"
        );
    }

    #[test]
    fn all_problems_are_reported_together() {
        let err = Config::from_toml(
            r#"
[[plugins]]
name = "a"
builtin = "x"
command = ["y"]

[[plugins]]
name = "a"
runtime = "node"

[[plugins]]
name = "c"
builtin = "x"
transport = "stdio"
module = "m"
"#,
        )
        .unwrap_err();
        let ConfigError::Invalid(problems) = &err else {
            panic!("{err}")
        };
        let rendered: Vec<String> = problems
            .iter()
            .map(|p| format!("{}: {}", p.at, p.message))
            .collect();
        assert_eq!(
            rendered,
            [
                "plugins[0]: set exactly one of `builtin`, `command` or `runtime`",
                "plugins[1]: duplicate plugin name `a`",
                "plugins[1]: `runtime = \"node\"` needs `entry`",
                "plugins[2]: `module` needs `runtime = \"python\"`",
                "plugins[2]: `transport` does not apply to builtin plugins, which run in-process",
            ]
        );
    }

    #[test]
    fn empty_command_is_rejected() {
        let err = Config::from_toml("[[plugins]]\nname = \"p\"\ncommand = []\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid config file:\n  plugins[0]: `command` must name a program"
        );
    }

    #[test]
    fn overrides_apply_in_layer_order() {
        let mut config = Config::from_toml(FULL).unwrap();
        config.apply(Overrides::from_env([
            ("LSDOCS_OUTPUT_DIR", "from-env"),
            ("LSDOCS_BASE_URL", "/env/"),
            ("UNRELATED", "ignored"),
        ]));
        config.apply(Overrides {
            output_dir: Some("from-cli".into()),
            ..Overrides::default()
        });
        assert_eq!(config.source_dir, "content");
        assert_eq!(config.output_dir, "from-cli");
        assert_eq!(config.base_url.as_deref(), Some("/env/"));
    }
}
