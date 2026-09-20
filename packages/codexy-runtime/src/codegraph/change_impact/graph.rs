use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use super::super::Graph;
use super::model::{UnknownArea, UnknownReason};

#[derive(Debug)]
pub(super) struct SnapshotGraph {
    pub(super) reverse: BTreeMap<String, Vec<String>>,
    pub(super) unsupported: BTreeSet<String>,
    pub(super) unresolved: BTreeMap<String, Vec<String>>,
    pub(super) errors: BTreeMap<String, Vec<String>>,
    pub(super) truncated: bool,
}

#[derive(Debug)]
pub(super) struct ReversePaths {
    pub(super) paths: Vec<Vec<String>>,
    pub(super) truncated: bool,
}

impl SnapshotGraph {
    pub(super) fn from_graph(graph: Graph) -> Self {
        let unsupported = graph
            .files
            .iter()
            .filter(|file| !is_supported_path(&file.path))
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        let mut reverse = BTreeMap::<String, Vec<String>>::new();
        let mut unresolved = BTreeMap::<String, Vec<String>>::new();
        for edge in graph.edges {
            if edge.resolved {
                reverse.entry(edge.to).or_default().push(edge.from);
            } else {
                unresolved
                    .entry(edge.from)
                    .or_default()
                    .push(edge.specifier);
            }
        }
        sort_values(&mut reverse);
        sort_values(&mut unresolved);
        let mut errors = BTreeMap::<String, Vec<String>>::new();
        for error in graph.errors {
            errors.entry(error.path).or_default().push(error.message);
        }
        sort_values(&mut errors);
        Self {
            reverse,
            unsupported,
            unresolved,
            errors,
            truncated: graph.truncated,
        }
    }

    pub(super) fn reverse_paths(&self, start: &str, limit: usize) -> ReversePaths {
        let mut queue = VecDeque::from([(start.to_owned(), vec![start.to_owned()])]);
        let mut visited = BTreeSet::from([start.to_owned()]);
        let mut paths = Vec::new();
        let mut truncated = false;
        while let Some((current, path)) = queue.pop_front() {
            for dependent in self.reverse.get(&current).into_iter().flatten() {
                if !visited.insert(dependent.clone()) {
                    continue;
                }
                if paths.len() >= limit {
                    truncated = true;
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(dependent.clone());
                paths.push(next_path.clone());
                if is_supported_path(dependent) {
                    queue.push_back((dependent.clone(), next_path));
                }
            }
        }
        ReversePaths { paths, truncated }
    }

    pub(super) fn unknown_for_path(&self, path: &str) -> Vec<UnknownArea> {
        let mut unknown = Vec::new();
        if self.unsupported.contains(path) {
            unknown.push(UnknownArea {
                reason: UnknownReason::UnsupportedLanguage,
                path: Some(path.to_owned()),
                detail: "impact analysis supports Python and Rust only".to_owned(),
            });
        }
        if let Some(specifiers) = self.unresolved.get(path) {
            for specifier in specifiers {
                unknown.push(UnknownArea {
                    reason: UnknownReason::UnresolvedImport,
                    path: Some(path.to_owned()),
                    detail: format!("unresolved import: {specifier}"),
                });
            }
        }
        if let Some(messages) = self.errors.get(path) {
            for message in messages {
                unknown.push(UnknownArea {
                    reason: UnknownReason::ParseFailure,
                    path: Some(path.to_owned()),
                    detail: message.clone(),
                });
            }
        }
        unknown
    }
}

pub(super) fn is_supported_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "py" | "rs"))
}

fn sort_values(values: &mut BTreeMap<String, Vec<String>>) {
    for entries in values.values_mut() {
        entries.sort();
        entries.dedup();
    }
}
