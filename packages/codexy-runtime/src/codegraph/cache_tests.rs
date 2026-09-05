use std::fs;

use serde_json::json;

use super::build_graph;
use super::cache::{MAX_CACHE_BYTES, invalidate};
use super::parse::{parse_call_count, reset_parse_call_count};
use super::tools::call_tool;

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
fn indexed_file_additions_reparse_imports_that_depend_on_membership()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let package = repository.path().join("pkg");
    fs::create_dir(&package)?;
    fs::write(package.join("__init__.py"), "")?;
    fs::write(repository.path().join("entry.py"), "from pkg import sub\n")?;
    invalidate();

    let before = build_graph(repository.path(), Some(10));
    let before_entry = before
        .files
        .iter()
        .find(|file| file.path == "entry.py")
        .ok_or("entry before addition")?;
    assert_eq!(before_entry.imports, vec!["./pkg"]);

    fs::write(package.join("sub.py"), "VALUE = 1\n")?;
    let after = build_graph(repository.path(), Some(10));
    let after_entry = after
        .files
        .iter()
        .find(|file| file.path == "entry.py")
        .ok_or("entry after addition")?;
    assert_eq!(after_entry.imports, vec!["./pkg/sub"]);
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

#[test]
fn many_long_path_files_use_the_uncached_path() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let directories = 300;
    let files_per_directory = 50;
    for directory_index in 0..directories {
        let directory = repository
            .path()
            .join(format!("segment_{directory_index:03}_{}", "x".repeat(220)));
        let nested = directory.join(format!("nested_{}", "y".repeat(220)));
        fs::create_dir_all(&nested)?;
        for file_index in 0..files_per_directory {
            fs::write(
                nested.join(format!("file_{file_index:03}_{}.json", "z".repeat(220))),
                "",
            )?;
        }
    }
    invalidate();
    reset_parse_call_count();

    let _ = build_graph(repository.path(), Some(usize::MAX));
    let _ = build_graph(repository.path(), Some(usize::MAX));

    assert_eq!(
        parse_call_count(),
        directories * files_per_directory * 2,
        "retained cache state over the limit must use the uncached path"
    );
    Ok(())
}

#[test]
fn request_errors_clear_cache_before_the_next_valid_graph() -> Result<(), Box<dyn std::error::Error>>
{
    let repository = tempfile::tempdir()?;
    fs::write(
        repository.path().join("dep.rs"),
        "pub const VALUE: u8 = 1;\n",
    )?;
    fs::write(repository.path().join("entry.rs"), "mod dep;\n")?;
    invalidate();
    reset_parse_call_count();

    call_tool("codegraph_index", &json!({"root": repository.path()}))?;
    assert_eq!(parse_call_count(), 2);
    assert!(
        call_tool(
            "codegraph_reverse_deps",
            &json!({
                "root": repository.path()
            })
        )
        .is_err()
    );
    call_tool("codegraph_index", &json!({"root": repository.path()}))?;
    assert_eq!(parse_call_count(), 4);
    Ok(())
}
