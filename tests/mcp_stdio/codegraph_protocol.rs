use super::*;

#[test]
fn codegraph_stdio_indexes_searches_and_bounds_missing_neighbors()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("dep.rs"), "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(
        root.path().join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n",
    )?;
    std::fs::write(
        root.path().join("extra_one.rs"),
        "pub const ENTRY_ONE: u8 = 1;\n",
    )?;
    std::fs::write(
        root.path().join("extra_two.rs"),
        "pub const ENTRY_TWO: u8 = 2;\n",
    )?;

    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    let list = client.send(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "codegraph_index")
    );
    let index = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":root.path(),"limit":10}}
    }))?;
    let graph: Value = serde_json::from_str(
        index["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert!(
        graph["edges"]
            .as_array()
            .ok_or("edges must be array")?
            .iter()
            .any(|edge| edge["from"] == "entry.rs" && edge["to"] == "dep.rs")
    );
    let search = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{"root":root.path(),"query":"ENTRY","limit":1.0}}
    }))?;
    let search_text = search["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?;
    assert!(
        search_text.contains("ENTRY"),
        "codegraph_search must return a matching line, got {search_text:?}"
    );
    assert_eq!(
        search_text.lines().count(),
        1,
        "codegraph_search must stop at the requested line limit"
    );
    let missing = client.send(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"codegraph_neighbors","arguments":{"root":root.path(),"path":"missing.rs"}}
    }))?;
    let neighbors: Value = serde_json::from_str(
        missing["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(neighbors, json!([]));
    Ok(())
}

#[test]
fn codegraph_stdio_matches_absolute_paths_when_root_is_relative()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let dependency = root.path().join("dep.rs");
    let entry = root.path().join("entry.rs");
    std::fs::write(&dependency, "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(&entry, "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n")?;

    let mut client = McpClient::spawn_in(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"), root.path())?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let reverse_deps = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_reverse_deps","arguments":{"root":".","path":dependency,"limit":10}}
    }))?;
    let reverse_payload: Value = serde_json::from_str(
        reverse_deps["result"]["content"][0]["text"]
            .as_str()
            .ok_or("reverse deps text")?,
    )?;
    assert!(
        reverse_payload["dependents"]
            .as_array()
            .ok_or("reverse dependents must be array")?
            .iter()
            .any(|dependent| dependent["path"] == "entry.rs"),
        "absolute dependency path should match relative graph edges"
    );

    let neighborhood = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_neighborhood","arguments":{"root":".","path":entry,"depth":0.0,"limit":10.0}}
    }))?;
    let neighborhood_payload: Value = serde_json::from_str(
        neighborhood["result"]["content"][0]["text"]
            .as_str()
            .ok_or("neighborhood text")?,
    )?;
    let nodes = neighborhood_payload["nodes"]
        .as_array()
        .ok_or("neighborhood nodes must be array")?;
    assert!(nodes.iter().any(|node| node["path"] == "entry.rs"));
    assert!(
        !nodes.iter().any(|node| node["path"] == "dep.rs"),
        "float-encoded depth must be honored"
    );
    Ok(())
}

#[test]
fn codegraph_stdio_keeps_outside_absolute_paths_distinct() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let outside_dep = outside.path().join("dep.rs");
    std::fs::write(&outside_dep, "pub const OUTSIDE: u8 = 1;\n")?;
    let canonical_outside = outside_dep.canonicalize()?;
    let mirrored_dep = root.path().join(canonical_outside.strip_prefix("/")?);
    let mirrored_dir = mirrored_dep.parent().ok_or("mirrored parent")?;
    std::fs::create_dir_all(mirrored_dir)?;
    std::fs::write(
        &mirrored_dep,
        "mod leaf;\npub const MIRRORED: u8 = leaf::LEAF;\n",
    )?;
    std::fs::write(mirrored_dir.join("leaf.rs"), "pub const LEAF: u8 = 1;\n")?;
    std::fs::write(
        mirrored_dir.join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::MIRRORED;\n",
    )?;

    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let reverse_deps = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_reverse_deps","arguments":{"root":root.path(),"path":outside_dep,"limit":10}}
    }))?;
    let reverse_payload: Value = serde_json::from_str(
        reverse_deps["result"]["content"][0]["text"]
            .as_str()
            .ok_or("reverse deps text")?,
    )?;
    assert!(
        reverse_payload["dependents"]
            .as_array()
            .ok_or("reverse dependents must be array")?
            .is_empty(),
        "outside absolute path must not alias mirrored in-root reverse deps"
    );

    let neighborhood = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_neighborhood","arguments":{"root":root.path(),"path":outside_dep,"depth":1,"limit":10}}
    }))?;
    let neighborhood_payload: Value = serde_json::from_str(
        neighborhood["result"]["content"][0]["text"]
            .as_str()
            .ok_or("neighborhood text")?,
    )?;
    assert!(
        neighborhood_payload["edges"]
            .as_array()
            .ok_or("neighborhood edges must be array")?
            .is_empty(),
        "outside absolute path must not alias mirrored in-root neighborhood edges"
    );
    let nodes = neighborhood_payload["nodes"]
        .as_array()
        .ok_or("neighborhood nodes must be array")?;
    assert!(
        !nodes.iter().any(|node| {
            node["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("leaf.rs"))
        }),
        "outside absolute path must not traverse mirrored in-root imports"
    );
    Ok(())
}
