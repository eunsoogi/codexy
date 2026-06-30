use std::fs;

use codexy_runtime::codegraph::build_graph;

#[test]
fn codegraph_indexes_html_css_and_resolves_local_web_edges()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::create_dir_all(root.join("assets"))?;
    fs::write(
        root.join("index.html"),
        r#"<!doctype html>
<html>
  <head>
    <div></div>
    <link rel="stylesheet" href="./styles.css">
    <script type="module" src="./app.js"></script>
    <!-- <script src="./ghost.js"></script> -->
  </head>
  <body>
    <img src="./assets/logo.svg">
    <img srcset="./assets/small.svg 1x, ./assets/large.svg 2x">
  </body>
</html>
"#,
    )?;
    fs::write(
        root.join("styles.css"),
        r#"@import "./theme.css";
.logo { background: url("./assets/logo.svg"); }
/* .ghost { background: url("./assets/ghost.svg"); } */
"#,
    )?;
    fs::write(root.join("styles.scss"), "// @import \"./ghost.css\";\n")?;
    fs::write(
        root.join("styles.less"),
        "// .ghost { background: url(\"./ghost.css\"); }\n",
    )?;
    fs::write(root.join("styles.sass"), "// @import \"./ghost.css\"\n")?;
    fs::write(root.join("ghost.css"), ".ghost { color: red; }\n")?;
    fs::write(root.join("theme.css"), ":root { color: #111; }\n")?;
    fs::write(root.join("app.js"), "export const ready = true;\n")?;
    fs::write(root.join("ghost.js"), "export const ghost = true;\n")?;
    fs::write(root.join("assets/logo.svg"), "<svg viewBox=\"0 0 1 1\"/>\n")?;
    fs::write(
        root.join("assets/ghost.svg"),
        "<svg viewBox=\"0 0 1 1\"/>\n",
    )?;
    fs::write(
        root.join("assets/small.svg"),
        "<svg viewBox=\"0 0 1 1\"/>\n",
    )?;
    fs::write(
        root.join("assets/large.svg"),
        "<svg viewBox=\"0 0 1 1\"/>\n",
    )?;

    let graph = build_graph(root, Some(20));
    let files = graph
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    for expected in [
        "app.js",
        "assets/logo.svg",
        "index.html",
        "styles.css",
        "theme.css",
    ] {
        assert!(
            files.contains(&expected),
            "web file {expected} must be indexed, got {files:#?}"
        );
    }

    let resolved_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.resolved)
        .map(|edge| {
            (
                edge.from.as_str(),
                edge.to.as_str(),
                edge.specifier.as_str(),
            )
        })
        .collect::<Vec<_>>();
    for expected in [
        ("index.html", "app.js", "./app.js"),
        ("index.html", "assets/large.svg", "./assets/large.svg"),
        ("index.html", "assets/logo.svg", "./assets/logo.svg"),
        ("index.html", "assets/small.svg", "./assets/small.svg"),
        ("index.html", "styles.css", "./styles.css"),
        ("styles.css", "assets/logo.svg", "./assets/logo.svg"),
        ("styles.css", "theme.css", "./theme.css"),
    ] {
        assert!(
            resolved_edges.contains(&expected),
            "missing resolved web edge {expected:?}, got {resolved_edges:#?}"
        );
    }
    let all_edges = graph
        .edges
        .iter()
        .map(|edge| {
            (
                edge.from.as_str(),
                edge.to.as_str(),
                edge.specifier.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        !all_edges.iter().any(|(_, to, specifier)| *to == "ghost.js"
            || *to == "assets/ghost.svg"
            || *to == "ghost.css"
            || *specifier == "./ghost.js"
            || *specifier == "./assets/ghost.svg"
            || *specifier == "./ghost.css"),
        "commented HTML/CSS references must not produce edges, got {all_edges:#?}"
    );

    Ok(())
}
