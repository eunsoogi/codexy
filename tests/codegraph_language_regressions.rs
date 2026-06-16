use std::fs;

use codexy_runtime::codegraph::{build_graph, neighborhood, reverse_deps};

#[test]
fn codegraph_go_import_blocks_ignore_commented_local_imports()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::create_dir_all(root.join("pkg/live"))?;
    fs::create_dir_all(root.join("pkg/commented"))?;
    fs::write(root.join("go.mod"), "module example.com/acme/app\n")?;
    fs::write(
        root.join("main.go"),
        r#"package main

import (
    "example.com/acme/app/pkg/live"
    /*
    "example.com/acme/app/pkg/commented"
    */
)

func main() {
    live.Run()
}
"#,
    )?;
    fs::write(
        root.join("pkg/live/live.go"),
        "package live\nfunc Run() {}\n",
    )?;
    fs::write(
        root.join("pkg/commented/commented.go"),
        "package commented\nfunc Run() {}\n",
    )?;

    let graph = build_graph(root, Some(10));
    let main_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.from == "main.go")
        .map(|edge| edge.to.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        main_edges,
        vec!["pkg/live/live.go"],
        "commented Go import-block entries must stay masked, got {main_edges:#?}"
    );

    Ok(())
}

#[test]
fn codegraph_neighborhood_reports_truncated_when_node_limit_is_reached()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("entry.rs"),
        "mod a;\nmod b;\npub const VALUE: u8 = 1;\n",
    )?;
    fs::write(root.join("a.rs"), "pub const A: u8 = 1;\n")?;
    fs::write(root.join("b.rs"), "pub const B: u8 = 2;\n")?;

    let result = neighborhood(root, "entry.rs", Some(1), Some(1));

    assert_eq!(result.nodes.len(), 1);
    assert!(
        result.truncated,
        "neighborhood should report truncation when queued reachable nodes exceed limit"
    );

    Ok(())
}

#[test]
fn codegraph_reverse_deps_preserves_escaping_target_paths() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let outside = tempfile::tempdir()?;
    fs::write(root.join("dep.rs"), "pub const VALUE: u8 = 1;\n")?;
    fs::write(
        root.join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n",
    )?;
    let outside_dep = outside.path().join("dep.rs");
    fs::write(&outside_dep, "pub const OUTSIDE: u8 = 1;\n")?;
    let canonical_outside = outside_dep.canonicalize()?;
    let mirrored_inside = root.join(canonical_outside.strip_prefix("/")?);
    fs::create_dir_all(mirrored_inside.parent().ok_or("mirrored parent")?)?;
    fs::write(
        &mirrored_inside,
        "mod leaf;\npub const MIRRORED: u8 = leaf::LEAF;\n",
    )?;
    fs::write(
        mirrored_inside
            .parent()
            .ok_or("mirrored leaf parent")?
            .join("leaf.rs"),
        "pub const LEAF: u8 = 1;\n",
    )?;
    fs::write(
        mirrored_inside
            .parent()
            .ok_or("mirrored entry parent")?
            .join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::MIRRORED;\n",
    )?;

    let escaped_relative = reverse_deps(root, "../dep.rs", Some(10));
    assert_eq!(
        escaped_relative.path, "../dep.rs",
        "leading parent path segments must be preserved"
    );
    assert!(
        escaped_relative.dependents.is_empty(),
        "escaping relative target must not alias in-root dep.rs"
    );

    let escaped_absolute = reverse_deps(root, &outside_dep.to_string_lossy(), Some(10));
    assert!(
        escaped_absolute.dependents.is_empty(),
        "outside absolute target must not alias mirrored in-root path"
    );

    let escaped_neighborhood = neighborhood(root, "../dep.rs", Some(1), Some(10));
    assert_eq!(
        escaped_neighborhood.path, "../dep.rs",
        "neighborhood must preserve leading parent path segments"
    );
    assert!(
        !escaped_neighborhood
            .nodes
            .iter()
            .any(|node| node.path == "dep.rs"),
        "escaping neighborhood target must not alias in-root dep.rs"
    );

    let absolute_neighborhood =
        neighborhood(root, &outside_dep.to_string_lossy(), Some(1), Some(10));
    assert!(
        absolute_neighborhood.nodes.len() <= 1,
        "outside absolute neighborhood must not traverse mirrored in-root imports"
    );
    assert!(
        absolute_neighborhood.edges.is_empty(),
        "outside absolute neighborhood must not alias mirrored in-root edges"
    );

    Ok(())
}
