use std::path::Path;

use serde::Serialize;
use serde_json::{Value, json};

use super::super::errors::{begin_operation, take_errors};
use super::super::files::{read_source, result_limit, walk_code_files};

const OVERVIEW_EDGE_LIMIT: usize = 300;
const OVERVIEW_IMPORT_LIMIT: usize = 80;

#[derive(Debug, Serialize)]
pub(super) struct ImportLine {
    line: usize,
    text: String,
}

pub(super) fn overview(root: &Path, limit: Option<usize>) -> Value {
    begin_operation();
    let file_limit = result_limit(limit);
    let all_files = walk_code_files(root);
    let files = all_files[..file_limit.min(all_files.len())].to_vec();
    let file_truncated = all_files.len() > files.len();
    let mut errors = take_errors();
    let total_files_known = errors.is_empty();
    let mut imports_truncated = false;
    let mut import_edges = files
        .iter()
        .flat_map(|file| {
            let (imports, truncated) = imports_for(root, file);
            imports_truncated |= truncated;
            imports
                .into_iter()
                .map(move |edge| json!({"file": file, "line": edge.line, "text": edge.text}))
        })
        .take(OVERVIEW_EDGE_LIMIT + 1)
        .collect::<Vec<_>>();
    let edges_truncated = import_edges.len() > OVERVIEW_EDGE_LIMIT;
    import_edges.truncate(OVERVIEW_EDGE_LIMIT);
    errors.extend(take_errors());
    let mut result = json!({
        "root": root,
        "fileCount": files.len(),
        "files": files,
        "importEdges": import_edges,
        "limits": {"files": file_limit, "edges": OVERVIEW_EDGE_LIMIT, "importsPerFile": OVERVIEW_IMPORT_LIMIT},
        "totals": {"files": total_files_known.then_some(all_files.len()), "edges": (!file_truncated && !edges_truncated && !imports_truncated && errors.is_empty()).then_some(import_edges.len())},
        "truncation": {"files": file_truncated, "edges": edges_truncated, "importsPerFile": imports_truncated}
    });
    if !errors.is_empty() {
        result["partial"] = json!(true);
        result["errors"] = json!(errors);
    }
    result
}

pub(super) fn imports_for(root: &Path, file_path: &str) -> (Vec<ImportLine>, bool) {
    let mut imports = read_source(root, file_path)
        .lines()
        .enumerate()
        .map(|(index, line)| ImportLine {
            line: index + 1,
            text: line.trim().to_owned(),
        })
        .filter(|line| {
            ["import ", "from ", "use "]
                .iter()
                .any(|prefix| line.text.starts_with(prefix))
                || line.text.contains("require(")
        })
        .take(OVERVIEW_IMPORT_LIMIT + 1)
        .collect::<Vec<_>>();
    let truncated = imports.len() > OVERVIEW_IMPORT_LIMIT;
    imports.truncate(OVERVIEW_IMPORT_LIMIT);
    (imports, truncated)
}
