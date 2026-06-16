use std::fs;

use codexy_runtime::codegraph::{build_graph, neighborhood};

#[test]
fn codegraph_dynamic_imports_inside_template_expressions_create_edges()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a dynamic import inside a template expression and import-like text decoys.
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("entry.js"),
        r#"export async function loadChunk() {
  return `${// import("./fake-comment.js")
 import("./chunk.js", { with: { type: "javascript" } })}`;
}
const staticText = "import { fakeStatic } from \"./fake-static.js\"";
const requireText = 'const fakeRequire = require("./fake-require.js")';
const dynamicText = 'import("./fake-dynamic.js")';
const nestedTemplateText = `${`nested`} import("./fake-nested.js")`;
const importPattern = /import\s+["']\.\/fake-regex\.js["']/;
"#,
    )?;
    fs::write(root.join("chunk.js"), "export const chunk = 1;\n")?;
    fs::write(
        root.join("fake-static.js"),
        "export const fakeStatic = 2;\n",
    )?;
    fs::write(
        root.join("fake-require.js"),
        "export const fakeRequire = 3;\n",
    )?;
    fs::write(
        root.join("fake-dynamic.js"),
        "export const fakeDynamic = 4;\n",
    )?;
    fs::write(
        root.join("fake-comment.js"),
        "export const fakeComment = 5;\n",
    )?;
    fs::write(root.join("fake-regex.js"), "export const fakeRegex = 6;\n")?;

    // When: the Rust codegraph indexes the fixture.
    let graph = build_graph(root, Some(10));
    let entry_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.from == "entry.js")
        .collect::<Vec<_>>();

    // Then: only the executable dynamic import creates an edge.
    assert_eq!(
        entry_edges
            .iter()
            .map(|edge| edge.to.as_str())
            .collect::<Vec<_>>(),
        vec!["chunk.js"],
        "expected only the template-expression dynamic import edge, got {entry_edges:#?}"
    );

    Ok(())
}

#[test]
fn codegraph_named_reexport_aliases_preserve_exported_names()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("named.js"),
        "export { leaf as renamedLeaf } from \"./leaf.js\";\n",
    )?;
    fs::write(
        root.join("compact-reexport.js"),
        "export {leaf as compactLeaf} from \"./leaf.js\";\n",
    )?;
    fs::write(root.join("leaf.js"), "export const leaf = 1;\n")?;

    let graph = build_graph(root, Some(10));
    let exports_for = |path: &str| {
        graph
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.exports.clone())
            .unwrap_or_default()
    };

    assert_eq!(exports_for("named.js"), vec!["renamedLeaf"]);
    assert_eq!(exports_for("compact-reexport.js"), vec!["compactLeaf"]);

    Ok(())
}

#[test]
fn codegraph_regex_literals_do_not_create_import_edges() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("entry.js"),
        r#"const pattern = /import "./fake.js"/;
export function matchImport() {
  return /import "./fake-return.js"/;
}
export const value = pattern.test("import \"./fake.js\"");
"#,
    )?;
    fs::write(root.join("fake.js"), "export const fake = true;\n")?;
    fs::write(
        root.join("fake-return.js"),
        "export const fakeReturn = true;\n",
    )?;

    let graph = build_graph(root, Some(10));
    let entry_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.from == "entry.js")
        .collect::<Vec<_>>();

    assert!(
        entry_edges.is_empty(),
        "regex literal import-like text should stay masked, got {entry_edges:#?}"
    );

    Ok(())
}

#[test]
fn codegraph_neighborhood_reports_truncated_when_node_limit_is_reached()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("entry.js"),
        "import { a } from \"./a.js\";\nimport { b } from \"./b.js\";\nexport const value = a + b;\n",
    )?;
    fs::write(root.join("a.js"), "export const a = 1;\n")?;
    fs::write(root.join("b.js"), "export const b = 2;\n")?;

    let result = neighborhood(root, "entry.js", Some(1), Some(1));

    assert_eq!(result.nodes.len(), 1);
    assert!(
        result.truncated,
        "neighborhood should report truncation when queued reachable nodes exceed limit"
    );

    Ok(())
}

#[test]
fn codegraph_python_bare_relative_imports_keep_unresolved_edges()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::create_dir_all(root.join("pkg"))?;
    fs::write(root.join("pkg/__init__.py"), "")?;
    fs::write(root.join("pkg/module.py"), "from . import missing\n")?;

    let graph = build_graph(root, Some(10));
    let edge = graph
        .edges
        .iter()
        .find(|edge| edge.from == "pkg/module.py")
        .expect("bare relative import should create an edge");

    assert_eq!(edge.to, "./missing");
    assert!(
        !edge.resolved,
        "missing bare relative import should remain visible as unresolved"
    );

    Ok(())
}
