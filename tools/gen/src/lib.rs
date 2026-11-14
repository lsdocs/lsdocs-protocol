//! Generates the JSON Schemas in `schema/` and the golden fixtures in
//! `fixtures/`.
//!
//! Every fixture is written twice from the same example value: as
//! MessagePack (`.msgpack`) and as JSON (`.json`). `nur test fixtures` decodes
//! the MessagePack with nushell's own decoder and checks it matches the JSON,
//! which catches structs accidentally encoded as positional arrays.
//!
//! The `generated_files_are_up_to_date` test fails when the checked-in files
//! differ from what this crate generates.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use lsdocs_config::ConfigFile;
use lsdocs_doctree::Document;
use lsdocs_protocol::{Notification, Request, msgpack, notification, request};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};

mod examples;

/// Directories whose contents this crate owns, relative to the workspace root.
pub const MANAGED_DIRS: [&str; 2] = ["schema", "fixtures"];

/// A file to generate.
#[derive(Debug)]
pub struct Output {
    /// Path relative to the workspace root, with `/` separators.
    pub path: String,
    /// File contents.
    pub bytes: Vec<u8>,
}

/// The workspace root.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every generated file.
pub fn outputs() -> Vec<Output> {
    let mut out = Vec::new();

    schema::<Document>(&mut out, "schema/doctree/document.json");
    schema::<ConfigFile>(&mut out, "schema/config/lsdocs.toml.json");
    fixture(&mut out, "fixtures/doctree/document", &examples::document());

    let ex = examples::Messages::new();
    request::<request::Initialize>(&mut out, ex.initialize_params, ex.initialize_result);
    request::<request::Shutdown>(&mut out, (), ());
    request::<request::Parse>(&mut out, ex.parse_params, ex.parse_result);
    request::<request::TransformCollect>(&mut out, ex.collect_params, ex.collect_result);
    request::<request::TransformApply>(&mut out, ex.apply_params, ex.apply_result);
    request::<request::Render>(&mut out, ex.render_params, ex.render_result);
    request::<request::RenderSite>(&mut out, ex.render_site_params, ex.render_site_result);
    notification::<notification::Initialized>(&mut out, ());
    notification::<notification::Exit>(&mut out, ());
    notification::<notification::Cancel>(&mut out, ex.cancel);
    notification::<notification::Log>(&mut out, ex.log);
    notification::<notification::Progress>(&mut out, ex.progress);
    notification::<notification::Diagnostics>(&mut out, ex.diagnostics);

    out
}

/// Writes every output and deletes stale files in the managed directories.
pub fn write(root: &Path) -> io::Result<()> {
    let outputs = outputs();
    for output in &outputs {
        let path = root.join(&output.path);
        fs::create_dir_all(path.parent().expect("outputs live in a directory"))?;
        fs::write(path, &output.bytes)?;
    }
    for extra in extra_files(root, &outputs)? {
        fs::remove_file(root.join(extra))?;
    }
    Ok(())
}

/// Paths (relative to `root`) of generated files that are missing, differ,
/// or should no longer exist.
pub fn stale(root: &Path) -> io::Result<Vec<String>> {
    let outputs = outputs();
    let mut stale = Vec::new();
    for output in &outputs {
        match fs::read(root.join(&output.path)) {
            Ok(bytes) if bytes == output.bytes => {}
            Ok(_) => stale.push(output.path.clone()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => stale.push(output.path.clone()),
            Err(e) => return Err(e),
        }
    }
    stale.extend(extra_files(root, &outputs)?);
    Ok(stale)
}

fn extra_files(root: &Path, outputs: &[Output]) -> io::Result<Vec<String>> {
    let expected: BTreeSet<&str> = outputs.iter().map(|o| o.path.as_str()).collect();
    let mut extra = Vec::new();
    for dir in MANAGED_DIRS {
        for path in walk(&root.join(dir))? {
            let rel = path
                .strip_prefix(root)
                .expect("walked under root")
                .to_string_lossy()
                .replace('\\', "/");
            if !expected.contains(rel.as_str()) {
                extra.push(rel);
            }
        }
    }
    Ok(extra)
}

fn walk(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(files),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(walk(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

/// File stem for a method name: `transform/collect` -> `transform.collect`,
/// `$/cancel` -> `cancel`.
fn stem(method: &str) -> String {
    method.trim_start_matches("$/").replace('/', ".")
}

fn pretty_json<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("serializable");
    bytes.push(b'\n');
    bytes
}

fn schema<T: JsonSchema>(out: &mut Vec<Output>, path: &str) {
    out.push(Output {
        path: path.into(),
        bytes: pretty_json(&schemars::schema_for!(T)),
    });
}

/// Adds `<base>.msgpack` and `<base>.json` for `value`, after checking the
/// MessagePack decodes back to the same value.
fn fixture<T>(out: &mut Vec<Output>, base: &str, value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = msgpack::encode(value).expect("encodable");
    let decoded: T = msgpack::decode(&bytes).expect("decodable");
    assert_eq!(&decoded, value, "{base} does not round-trip");
    out.push(Output {
        path: format!("{base}.msgpack"),
        bytes,
    });
    out.push(Output {
        path: format!("{base}.json"),
        bytes: pretty_json(value),
    });
}

/// Schema and fixture for a message part, skipped when it is `()`.
fn part<T>(out: &mut Vec<Output>, base: &str, value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug + JsonSchema,
{
    if serde_json::to_value(value).expect("serializable").is_null() {
        return;
    }
    schema::<T>(out, &format!("schema/protocol/{base}.json"));
    fixture(out, &format!("fixtures/protocol/{base}"), value);
}

fn request<R>(out: &mut Vec<Output>, params: R::Params, result: R::Result)
where
    R: Request,
    R::Params: PartialEq + Debug + JsonSchema,
    R::Result: PartialEq + Debug + JsonSchema,
{
    let stem = stem(R::METHOD);
    part(out, &format!("{stem}.params"), &params);
    part(out, &format!("{stem}.result"), &result);
}

fn notification<N>(out: &mut Vec<Output>, params: N::Params)
where
    N: Notification,
    N::Params: PartialEq + Debug + JsonSchema,
{
    part(out, &format!("{}.params", stem(N::METHOD)), &params);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_files_are_up_to_date() {
        let stale = stale(&workspace_root()).unwrap();
        assert!(
            stale.is_empty(),
            "generated files are out of date; run `nur gen`:\n  {}",
            stale.join("\n  ")
        );
    }

    #[test]
    fn example_document_covers_every_node_kind() {
        let kinds: BTreeSet<String> = examples::document()
            .descendants()
            .map(|n| n.kind().into_owned())
            .collect();
        let expected: BTreeSet<String> = [
            "section",
            "heading",
            "paragraph",
            "list",
            "list_item",
            "block_quote",
            "code_block",
            "table",
            "table_row",
            "table_cell",
            "thematic_break",
            "raw",
            "text",
            "emphasis",
            "strong",
            "inline_code",
            "link",
            "image",
            "line_break",
            "custom:admonition",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(kinds, expected);
    }

    #[test]
    fn method_stems() {
        assert_eq!(stem("transform/collect"), "transform.collect");
        assert_eq!(stem("$/cancel"), "cancel");
        assert_eq!(stem("parse"), "parse");
    }
}
