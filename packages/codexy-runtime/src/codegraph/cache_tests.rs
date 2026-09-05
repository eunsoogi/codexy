use std::fs;

use super::build_graph;
use super::cache::{MAX_CACHE_BYTES, invalidate};
use super::parse::{parse_call_count, reset_parse_call_count};

#[test]
fn repeated_graph_builds_reuse_each_parsed_file() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    fs::write(
        repository.path().join("dep.rs"),
        "pub const VALUE: u8 = 1;\n",
    )?;
    fs::write(
        repository.path().join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n",
    )?;
    reset_parse_call_count();

    let first = build_graph(repository.path(), Some(10));
    for _ in 0..9 {
        let graph = build_graph(repository.path(), Some(10));
        assert_eq!(graph.files.len(), 2);
    }

    assert_eq!(first.files.len(), 2);
    assert_eq!(
        parse_call_count(),
        2,
        "identical requests should parse once"
    );
    Ok(())
}

#[test]
fn content_digest_refreshes_same_size_file() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let dependency = repository.path().join("dep.rs");
    fs::write(&dependency, "pub const VALUE: u8 = 1;\n")?;
    fs::write(repository.path().join("entry.rs"), "mod dep;\n")?;
    invalidate();
    reset_parse_call_count();

    let _ = build_graph(repository.path(), Some(10));
    fs::write(&dependency, "pub const OTHER: u8 = 1;\n")?;
    let graph = build_graph(repository.path(), Some(10));

    let dependency_graph = graph
        .files
        .iter()
        .find(|file| file.path == "dep.rs")
        .ok_or("dependency graph file")?;
    assert_eq!(dependency_graph.exports, vec!["OTHER"]);
    assert_eq!(
        parse_call_count(),
        3,
        "only the changed file should reparse"
    );
    Ok(())
}

#[test]
fn root_and_ignore_changes_do_not_reuse_stale_graphs() -> Result<(), Box<dyn std::error::Error>> {
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    fs::write(first.path().join("entry.rs"), "pub const FIRST: u8 = 1;\n")?;
    fs::write(
        second.path().join("entry.rs"),
        "pub const SECOND: u8 = 2;\n",
    )?;
    fs::write(
        second.path().join("ignored.rs"),
        "pub const IGNORED: u8 = 3;\n",
    )?;
    fs::create_dir(second.path().join(".git"))?;
    invalidate();
    reset_parse_call_count();

    let first_graph = build_graph(first.path(), Some(10));
    let second_graph = build_graph(second.path(), Some(10));
    fs::write(second.path().join(".gitignore"), "ignored.rs\n")?;
    let ignored_graph = build_graph(second.path(), Some(10));

    assert_eq!(first_graph.files[0].exports, vec!["FIRST"]);
    assert_eq!(second_graph.files.len(), 2);
    assert_eq!(second_graph.files[0].exports, vec!["SECOND"]);
    assert_eq!(ignored_graph.files.len(), 1);
    assert_eq!(ignored_graph.files[0].path, "entry.rs");
    Ok(())
}

#[test]
fn deleting_a_file_invalidates_the_previous_file_set() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let deleted = repository.path().join("deleted.rs");
    fs::write(&deleted, "pub const DELETED: u8 = 1;\n")?;
    fs::write(
        repository.path().join("entry.rs"),
        "pub const ENTRY: u8 = 2;\n",
    )?;
    invalidate();

    let before = build_graph(repository.path(), Some(10));
    fs::remove_file(deleted)?;
    let after = build_graph(repository.path(), Some(10));

    assert_eq!(before.files.len(), 2);
    assert_eq!(after.files.len(), 1);
    assert_eq!(after.files[0].path, "entry.rs");
    Ok(())
}

#[test]
fn oversized_source_uses_the_uncached_path() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let source = format!("//{}\n", "x".repeat(MAX_CACHE_BYTES + 1));
    fs::write(repository.path().join("large.rs"), source)?;
    invalidate();
    reset_parse_call_count();

    let _ = build_graph(repository.path(), Some(10));
    let _ = build_graph(repository.path(), Some(10));

    assert_eq!(
        parse_call_count(),
        2,
        "oversized entries must not be cached"
    );
    Ok(())
}
