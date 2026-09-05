use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use crate::codegraph::{build_graph, neighborhood, reverse_deps};
use crate::mcp::{ToolDef, text_result};

use super::cache::invalidate;
use super::errors::{CodegraphError, CodegraphErrorKind, begin_operation, take_errors};
use super::files::{read_source, repo_root, result_limit, walk_code_files};
use super::search::search;

#[must_use]
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "codegraph_overview",
            "Summarize code files and import edges for a repository root.",
            json!({"type":"object","properties":{"root":{"type":"string"},"limit":{"type":"number"}}}),
        ),
        ToolDef::new(
            "codegraph_search",
            "Search repository code and return path:line matches.",
            json!({"type":"object","properties":{"root":{"type":"string"},"query":{"type":"string"},"limit":{"type":"number"}},"required":["query"]}),
        ),
        ToolDef::new(
            "codegraph_neighbors",
            "Return import-like dependency lines for one source file.",
            json!({"type":"object","properties":{"root":{"type":"string"},"path":{"type":"string"}},"required":["path"]}),
        ),
        ToolDef::new(
            "codegraph_index",
            "Build a bounded code graph with import, export, edge, and truncation metadata.",
            json!({"type":"object","properties":{"root":{"type":"string"},"limit":{"type":"number"}}}),
        ),
        ToolDef::new(
            "codegraph_reverse_deps",
            "Return files that import a target path.",
            json!({"type":"object","properties":{"root":{"type":"string"},"path":{"type":"string"},"limit":{"type":"number"}},"required":["path"]}),
        ),
        ToolDef::new(
            "codegraph_neighborhood",
            "Return a bounded dependency neighborhood around one source file.",
            json!({"type":"object","properties":{"root":{"type":"string"},"path":{"type":"string"},"depth":{"type":"number"},"limit":{"type":"number"}},"required":["path"]}),
        ),
    ]
}

/// Calls a codegraph MCP tool by name.
///
/// # Errors
///
/// Returns an error when required arguments are missing, a search regex is invalid, JSON
/// serialization fails, or the tool name is unknown.
pub fn call_tool(name: &str, args: &Value) -> Result<Value> {
    let root_input = match root_argument(args) {
        Ok(root) => root,
        Err(error) => {
            invalidate();
            return Err(error);
        }
    };
    let root = match repo_root(root_input) {
        Ok(root) => root,
        Err(error) => {
            invalidate();
            return Err(error.into());
        }
    };
    match name {
        "codegraph_overview" => text_json(&overview(&root, limit(args))),
        "codegraph_search" => {
            let query = string_arg(args, "query")?;
            Ok(text_result(&serde_json::to_string(&search(
                &root,
                query,
                limit(args),
            )?)?))
        }
        "codegraph_neighbors" => {
            begin_operation();
            let imports = imports_for(&root, string_arg(args, "path")?);
            let errors = take_errors();
            if errors.is_empty() {
                text_json(&imports)
            } else {
                text_json(&PartialImports {
                    imports,
                    partial: true,
                    errors,
                })
            }
        }
        "codegraph_index" => text_json(&build_graph(&root, limit(args))),
        "codegraph_reverse_deps" => {
            text_json(&reverse_deps(&root, string_arg(args, "path")?, limit(args)))
        }
        "codegraph_neighborhood" => text_json(&neighborhood(
            &root,
            string_arg(args, "path")?,
            args.get("depth").and_then(value_usize),
            limit(args),
        )),
        _ => bail!("Unknown tool: {name}"),
    }
}

#[derive(Debug, Serialize)]
struct ImportLine {
    line: usize,
    text: String,
}

#[derive(Debug, Serialize)]
struct PartialImports {
    imports: Vec<ImportLine>,
    partial: bool,
    errors: Vec<CodegraphError>,
}

#[derive(Debug, Serialize)]
struct ImportEdgeLine {
    file: String,
    line: usize,
    text: String,
}

#[derive(Debug, Serialize)]
struct Overview {
    root: PathBuf,
    #[serde(rename = "fileCount")]
    file_count: usize,
    files: Vec<String>,
    #[serde(rename = "importEdges")]
    import_edges: Vec<ImportEdgeLine>,
    #[serde(skip_serializing_if = "is_false")]
    partial: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<CodegraphError>,
}

fn overview(root: &Path, limit: Option<usize>) -> Overview {
    begin_operation();
    let files = walk_code_files(root)
        .into_iter()
        .take(result_limit(limit))
        .collect::<Vec<_>>();
    let import_edges = files
        .iter()
        .flat_map(|file| {
            imports_for(root, file)
                .into_iter()
                .map(|edge| ImportEdgeLine {
                    file: file.clone(),
                    line: edge.line,
                    text: edge.text,
                })
                .collect::<Vec<_>>()
        })
        .take(300)
        .collect::<Vec<_>>();
    let errors = take_errors();
    Overview {
        root: root.to_path_buf(),
        file_count: files.len(),
        files,
        import_edges,
        partial: !errors.is_empty(),
        errors,
    }
}

fn imports_for(root: &Path, file_path: &str) -> Vec<ImportLine> {
    let text = read_source(root, file_path);
    text.lines()
        .enumerate()
        .map(|(index, line)| ImportLine {
            line: index + 1,
            text: line.trim().to_owned(),
        })
        .filter(|line| {
            line.text.starts_with("import ")
                || line.text.starts_with("from ")
                || line.text.starts_with("use ")
                || line.text.contains("require(")
        })
        .take(80)
        .collect()
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn text_json<T: Serialize>(value: &T) -> Result<Value> {
    Ok(text_result(&serde_json::to_string_pretty(value)?))
}

fn string_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str> {
    args.get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{name} is required"))
}

fn root_argument(args: &Value) -> Result<Option<&str>> {
    let Some(value) = args.get("root") else {
        return Ok(None);
    };
    value.as_str().map(Some).ok_or_else(|| {
        anyhow::Error::new(CodegraphError {
            kind: CodegraphErrorKind::RootInvalid,
            path: "root".to_owned(),
            message: "repository root must be a string when provided".to_owned(),
        })
    })
}

fn limit(args: &Value) -> Option<usize> {
    args.get("limit").and_then(value_usize)
}

fn value_usize(value: &Value) -> Option<usize> {
    if let Some(number) = value.as_u64() {
        return usize::try_from(number).ok();
    }
    let number = value.as_f64()?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return None;
    }
    format!("{number:.0}").parse().ok()
}
