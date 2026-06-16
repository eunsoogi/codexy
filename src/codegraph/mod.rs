mod candidates;
mod files;
mod language;
mod mask;
mod parse;
mod path_ops;
mod python;
mod resolve;
pub mod tools;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use self::files::{result_limit, walk_code_files};
use self::parse::parse_file;
use self::resolve::{graph_path, resolve_import};

#[derive(Debug, Clone, Serialize)]
pub struct GraphFile {
    pub path: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub specifier: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphMetadata {
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Graph {
    pub root: PathBuf,
    pub files: Vec<GraphFile>,
    pub edges: Vec<GraphEdge>,
    #[serde(rename = "totalFiles")]
    pub total_files: usize,
    pub limit: usize,
    pub truncated: bool,
    pub metadata: GraphMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dependent {
    pub path: String,
    pub specifier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReverseDeps {
    pub root: PathBuf,
    pub path: String,
    pub dependents: Vec<Dependent>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Neighborhood {
    pub root: PathBuf,
    pub path: String,
    pub depth: usize,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub limit: usize,
    pub truncated: bool,
}

#[must_use]
pub fn build_graph(root: &Path, limit: Option<usize>) -> Graph {
    let bounded_limit = result_limit(limit);
    let all_files = walk_code_files(root);
    let selected_files = all_files
        .iter()
        .take(bounded_limit)
        .cloned()
        .collect::<Vec<_>>();
    let indexed_files = all_files.iter().cloned().collect::<BTreeSet<_>>();
    let files = selected_files
        .iter()
        .map(|file| parse_file(root, file, &indexed_files))
        .collect::<Vec<_>>();
    let edges = files
        .iter()
        .flat_map(|file| {
            file.imports.iter().map(|specifier| {
                let resolved = resolve_import(root, &file.path, specifier, &indexed_files);
                GraphEdge {
                    from: file.path.clone(),
                    to: resolved.to,
                    specifier: specifier.clone(),
                    resolved: resolved.resolved,
                }
            })
        })
        .collect::<Vec<_>>();
    let truncated = all_files.len() > selected_files.len();
    Graph {
        root: root.to_path_buf(),
        files,
        edges,
        total_files: all_files.len(),
        limit: bounded_limit,
        truncated,
        metadata: GraphMetadata { truncated },
    }
}

#[must_use]
pub fn reverse_deps(root: &Path, target_path: &str, limit: Option<usize>) -> ReverseDeps {
    let bounded_limit = result_limit(limit);
    let graph = build_graph(root, Some(usize::MAX));
    let target = graph_path(root, target_path);
    let dependents = graph
        .edges
        .iter()
        .filter(|edge| edge.resolved && edge.to == target)
        .map(|edge| Dependent {
            path: edge.from.clone(),
            specifier: edge.specifier.clone(),
        })
        .take(bounded_limit)
        .collect::<Vec<_>>();
    ReverseDeps {
        root: root.to_path_buf(),
        path: target,
        dependents,
        limit: bounded_limit,
    }
}

#[must_use]
pub fn neighborhood(
    root: &Path,
    start_path: &str,
    depth: Option<usize>,
    limit: Option<usize>,
) -> Neighborhood {
    let graph = build_graph(root, Some(usize::MAX));
    let bounded_depth = depth.unwrap_or(1);
    let bounded_limit = result_limit(limit);
    let start = graph_path(root, start_path);
    let mut seen = BTreeSet::new();
    let mut queue = std::collections::VecDeque::from([(start.clone(), 0usize)]);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut truncated = false;
    while let Some((current, current_depth)) = queue.pop_front() {
        if nodes.len() >= bounded_limit {
            truncated = true;
            continue;
        }
        if !seen.insert(current.clone()) {
            continue;
        }
        nodes.push(GraphNode {
            path: current.clone(),
        });
        if current_depth >= bounded_depth {
            continue;
        }
        for edge in graph
            .edges
            .iter()
            .filter(|candidate| candidate.from == current && candidate.resolved)
        {
            edges.push(edge.clone());
            if !seen.contains(&edge.to) {
                queue.push_back((edge.to.clone(), current_depth + 1));
            }
        }
    }
    let returned = nodes
        .iter()
        .map(|node| node.path.clone())
        .collect::<BTreeSet<_>>();
    let neighborhood_edges = edges
        .into_iter()
        .filter(|edge| returned.contains(&edge.from) && returned.contains(&edge.to))
        .collect::<Vec<_>>();
    truncated = truncated || queue.iter().any(|(path, _)| !seen.contains(path));
    truncated = truncated || neighborhood_edges.len() > bounded_limit;
    Neighborhood {
        root: root.to_path_buf(),
        path: start,
        depth: bounded_depth,
        nodes,
        edges: neighborhood_edges.into_iter().take(bounded_limit).collect(),
        limit: bounded_limit,
        truncated,
    }
}

#[must_use]
pub const fn server_name() -> &'static str {
    "codexy-codegraph"
}
