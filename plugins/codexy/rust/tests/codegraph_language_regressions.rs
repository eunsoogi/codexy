use std::fs;

use codexy_runtime::codegraph::build_graph;

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
