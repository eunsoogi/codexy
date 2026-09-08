use std::mem::size_of;
use std::path::Path;

use super::*;

#[test]
fn reset_releases_oversized_file_index_capacity() {
    let mut cache = ParseCache::default();
    cache.files = Vec::with_capacity(MAX_CACHE_BYTES / size_of::<String>() + 1);
    assert!(cache.files.capacity() * size_of::<String>() > MAX_CACHE_BYTES);
    cache.reset();
    assert_eq!(cache.files.capacity(), 0);
}

#[test]
fn prepare_rejects_an_oversized_file_index() {
    let file_count = 1024;
    let path_size = MAX_CACHE_BYTES / file_count + 64;
    let snapshot = FileSnapshot {
        files: (0..file_count)
            .map(|index| format!("{index:04}-{}", "x".repeat(path_size)))
            .collect(),
        environment_digest: [0; 32],
    };
    let mut cache = ParseCache::default();

    cache.prepare(Path::new("/virtual/repository"), &snapshot);

    assert!(cache.base_storage_size() > MAX_CACHE_BYTES);
    assert!(cache.uncached);
    let graph = GraphFile {
        path: snapshot.files[0].clone(),
        imports: Vec::new(),
        exports: Vec::new(),
    };
    cache.store(0, [0; 32], &graph, 0);
    assert!(cache.entries.is_empty());
    assert!(cache.lookup(0, &snapshot.files[0], &[0; 32]).is_none());
    cache.finish(false);
    assert!(cache.files.is_empty());
}
