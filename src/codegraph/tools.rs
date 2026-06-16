use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use crate::codegraph::{build_graph, neighborhood, reverse_deps};
use crate::mcp::{ToolDef, text_result};

use super::files::{is_code_file, repo_root, result_limit, walk_code_files};

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
            "Search repository code with ripgrep and return path:line matches.",
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
/// Returns an error when required arguments are missing, ripgrep fails, JSON
/// serialization fails, or the tool name is unknown.
pub fn call_tool(name: &str, args: &Value) -> Result<Value> {
    let root = repo_root(args.get("root").and_then(Value::as_str));
    match name {
        "codegraph_overview" => text_json(&overview(&root, limit(args))),
        "codegraph_search" => {
            let query = string_arg(args, "query")?;
            Ok(text_result(&rg_lines(&root, query, limit(args))?))
        }
        "codegraph_neighbors" => text_json(&imports_for(&root, string_arg(args, "path")?)),
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
}

fn overview(root: &Path, limit: Option<usize>) -> Overview {
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
    Overview {
        root: root.to_path_buf(),
        file_count: files.len(),
        files,
        import_edges,
    }
}

fn imports_for(root: &Path, file_path: &str) -> Vec<ImportLine> {
    let path = root.join(file_path);
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
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

fn rg_lines(root: &Path, query: &str, limit: Option<usize>) -> Result<String> {
    let bounded_limit = result_limit(limit);
    let mut child = Command::new("rg")
        .args([
            "--hidden",
            "--line-buffered",
            "-n",
            "--glob",
            "!node_modules",
            "--glob",
            "!.git",
            "-e",
            query,
            ".",
        ])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("running rg in {}", root.display()))?;
    let stdout = child.stdout.take().context("reading rg stdout")?;
    let mut lines = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.context("reading rg output")?;
        if is_code_file(Path::new(line.split(':').next().unwrap_or_default())) {
            lines.push(line);
            if lines.len() >= bounded_limit {
                let _ = child.kill();
                break;
            }
        }
    }
    let status = child
        .wait()
        .with_context(|| format!("waiting for rg in {}", root.display()))?;
    if !status.success() && status.code() != Some(1) && lines.is_empty() {
        return Ok(format!(
            "Command failed: rg --hidden -n --glob !node_modules --glob !.git -e {query}"
        ));
    }
    Ok(lines.join("\n"))
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
