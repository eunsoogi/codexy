use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;
use std::path::{Path, PathBuf};

use super::GraphFile;
use super::errors::{CodegraphError, take_errors};
use super::files::read_source_snapshot;
use super::parse::parse_file;
use super::snapshot::FileSnapshot;

pub(super) const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const CACHE_TREE_NODE_BYTES: usize = 256;

#[derive(Debug, Clone)]
struct CachedFile {
    digest: [u8; 32],
    graph: GraphFile,
    bytes: usize,
}

#[derive(Debug, Default)]
struct ParseCache {
    root: Option<PathBuf>,
    environment_digest: Option<[u8; 32]>,
    files: BTreeSet<String>,
    entries: BTreeMap<String, CachedFile>,
    bytes: usize,
    uncached: bool,
}

thread_local! {
    static CACHE: RefCell<ParseCache> = RefCell::new(ParseCache::default());
}

pub(super) fn parse_files(
    root: &Path,
    snapshot: &FileSnapshot,
    selected_files: &[String],
    indexed_files: &BTreeSet<String>,
) -> (Vec<GraphFile>, Vec<CodegraphError>) {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        cache.prepare(root, snapshot);
        let files = selected_files
            .iter()
            .map(|file| parse_one(&mut cache, root, file, indexed_files))
            .collect();
        let errors = take_errors();
        cache.finish(!errors.is_empty());
        (files, errors)
    })
}

pub(super) fn invalidate() {
    CACHE.with(|cell| cell.borrow_mut().reset());
}

fn parse_one(
    cache: &mut ParseCache,
    root: &Path,
    file: &str,
    indexed_files: &BTreeSet<String>,
) -> GraphFile {
    let Some(source) = read_source_snapshot(root, file) else {
        cache.remove(file);
        return empty_graph_file(file);
    };
    if let Some(graph) = cache.lookup(file, &source.digest) {
        return graph;
    }
    let graph = parse_file(root, file, indexed_files, &source.source);
    cache.store(file, source.digest, &graph, source.source.len());
    graph
}

impl ParseCache {
    fn prepare(&mut self, root: &Path, snapshot: &FileSnapshot) {
        let current_files = snapshot.files.iter().cloned().collect::<BTreeSet<_>>();
        let root_changed = self.root.as_deref() != Some(root);
        let environment_changed = self.environment_digest != Some(snapshot.environment_digest);
        let file_set_changed = self.files != current_files;
        let cache_changed = root_changed || environment_changed || file_set_changed;
        if cache_changed {
            self.clear_entries();
        }
        self.root = Some(root.to_path_buf());
        self.environment_digest = Some(snapshot.environment_digest);
        self.files = current_files;
        self.uncached = false;
        if cache_changed {
            self.bytes = self.base_storage_size();
        }
        if self.bytes > MAX_CACHE_BYTES {
            self.clear_entries();
            self.uncached = true;
        }
    }

    fn lookup(&mut self, file: &str, digest: &[u8; 32]) -> Option<GraphFile> {
        if self.uncached {
            return None;
        }
        if self
            .entries
            .get(file)
            .is_some_and(|entry| entry.digest == *digest)
        {
            return self.entries.get(file).map(|entry| entry.graph.clone());
        }
        self.remove(file);
        None
    }

    fn store(&mut self, file: &str, digest: [u8; 32], graph: &GraphFile, source_bytes: usize) {
        if self.uncached {
            return;
        }
        let key = file.to_owned();
        let bytes = cached_file_storage_size(&key, graph, source_bytes);
        self.remove(file);
        if self.bytes.saturating_add(bytes) > MAX_CACHE_BYTES {
            self.clear_entries();
            self.uncached = true;
            return;
        }
        self.bytes += bytes;
        self.entries.insert(
            key,
            CachedFile {
                digest,
                graph: graph.clone(),
                bytes,
            },
        );
    }

    fn remove(&mut self, file: &str) {
        if let Some(entry) = self.entries.remove(file) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
    }

    fn finish(&mut self, has_errors: bool) {
        if has_errors || self.uncached {
            self.reset();
        }
    }

    fn clear_entries(&mut self) {
        self.entries.clear();
        self.bytes = 0;
        self.uncached = false;
    }

    fn reset(&mut self) {
        self.root = None;
        self.environment_digest = None;
        self.files.clear();
        self.clear_entries();
    }

    fn base_storage_size(&self) -> usize {
        size_of::<Self>()
            .saturating_add(self.root.as_deref().map_or(0, |path| {
                size_of::<PathBuf>().saturating_add(path.to_string_lossy().len().saturating_mul(2))
            }))
            .saturating_add(self.files.iter().map(tree_entry_storage_size).sum())
    }
}

fn graph_storage_size(graph: &GraphFile, source_bytes: usize) -> usize {
    source_bytes
        .saturating_add(size_of::<CachedFile>())
        .saturating_add(owned_string_storage_size(&graph.path))
        .saturating_add(vector_storage_size(
            &graph.imports,
            graph.imports.capacity(),
        ))
        .saturating_add(vector_storage_size(
            &graph.exports,
            graph.exports.capacity(),
        ))
}

fn cached_file_storage_size(file: &String, graph: &GraphFile, source_bytes: usize) -> usize {
    CACHE_TREE_NODE_BYTES
        .saturating_add(owned_string_storage_size(file))
        .saturating_add(graph_storage_size(graph, source_bytes))
}

fn tree_entry_storage_size(file: &String) -> usize {
    CACHE_TREE_NODE_BYTES.saturating_add(owned_string_storage_size(file))
}

fn vector_storage_size(values: &[String], capacity: usize) -> usize {
    capacity
        .saturating_mul(size_of::<String>())
        .saturating_add(values.iter().map(owned_string_storage_size).sum())
}

fn owned_string_storage_size(value: &String) -> usize {
    size_of::<String>().saturating_add(value.capacity())
}

fn empty_graph_file(file: &str) -> GraphFile {
    GraphFile {
        path: file.to_owned(),
        imports: Vec::new(),
        exports: Vec::new(),
    }
}
