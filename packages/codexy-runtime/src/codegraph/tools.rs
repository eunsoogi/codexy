use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use crate::codegraph::{build_graph, neighborhood, reverse_deps};
use crate::mcp::{ToolDef, text_result};

use super::cache::invalidate;
use super::errors::{CodegraphError, CodegraphErrorKind, begin_operation, take_errors};
use super::files::repo_root;
use super::search::search;

mod impact;
mod legacy;
mod schema;
mod selection;

use impact::change_impact_result;
use legacy::{imports_for, overview};
use schema::{change_impact_schema, check_selection_schema};
use selection::check_selection_result;

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
        ToolDef::new(
            "codegraph_change_impact",
            "Collect a read-only change set and explain direct, transitive, and unknown impact.",
            change_impact_schema(),
        ),
        ToolDef::new(
            "codegraph_check_selection",
            "Recommend mapped checks from read-only change and impact evidence without executing them.",
            check_selection_schema(),
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
    let result = call_tool_inner(name, args);
    if result.is_err() {
        invalidate();
    }
    result
}

fn call_tool_inner(name: &str, args: &Value) -> Result<Value> {
    let root = repo_root(root_argument(args)?)?;
    match name {
        "codegraph_overview" => text_json(&overview(&root, limit(args))),
        "codegraph_search" => Ok(text_result(&serde_json::to_string(&search(
            &root,
            string_arg(args, "query")?,
            limit(args),
        )?)?)),
        "codegraph_neighbors" => {
            begin_operation();
            let (imports, _) = imports_for(&root, string_arg(args, "path")?);
            let errors = take_errors();
            if errors.is_empty() {
                text_json(&imports)
            } else {
                text_json(&json!({
                    "imports": imports,
                    "partial": true,
                    "errors": errors
                }))
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
        "codegraph_change_impact" => change_impact_result(&root, args),
        "codegraph_check_selection" => check_selection_result(&root, args),
        _ => bail!("Unknown tool: {name}"),
    }
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
