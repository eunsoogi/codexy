use super::*;

#[path = "codegraph_errors.rs"]
mod codegraph_errors;
#[path = "codegraph_root.rs"]
mod codegraph_root;

#[test]
fn codegraph_stdio_preserves_protocol_and_search_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let relative_root = tempfile::tempdir()?;
    let dependency = relative_root.path().join("dep.rs");
    let entry = relative_root.path().join("entry.rs");
    std::fs::write(&dependency, "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(&entry, "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n")?;

    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        relative_root.path(),
    )?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    assert_eq!(init["result"]["serverInfo"]["version"], codexy_runtime::version::runtime_version());
    let omitted_root = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"limit":10}}
    }))?;
    let omitted_graph: Value = serde_json::from_str(
        omitted_root["result"]["content"][0]["text"]
            .as_str()
            .ok_or("omitted-root text")?,
    )?;
    let actual_root = std::path::PathBuf::from(
        omitted_graph["root"]
            .as_str()
            .ok_or("omitted-root path")?,
    )
    .canonicalize()?;
    let expected_root = relative_root.path().canonicalize()?;
    assert_eq!(actual_root, expected_root);
    assert!(omitted_graph["partial"].is_null());
    assert!(omitted_graph["errors"].is_null());
    codegraph_stdio_indexes_searches_and_bounds_missing_neighbors(&mut client)?;
    codegraph_stdio_matches_absolute_paths_when_root_is_relative(
        &mut client,
        &dependency,
        &entry,
    )?;
    codegraph_errors::codegraph_stdio_keeps_outside_absolute_paths_distinct(&mut client)?;
    super::codegraph_search_bounds::search_bounds_cases(&mut client)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn codegraph_stdio_rejects_an_explicit_unreadable_root()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let repository = tempfile::tempdir()?;
    let unreadable = repository.path().join("unreadable-repository");
    std::fs::create_dir(&unreadable)?;
    let mut permissions = std::fs::metadata(&unreadable)?.permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&unreadable, permissions)?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":unreadable}}
    }))?;
    let mut restore = std::fs::metadata(&unreadable)?.permissions();
    restore.set_mode(0o700);
    std::fs::set_permissions(&unreadable, restore)?;
    assert_eq!(response["error"]["code"], -32000);
    assert!(response["error"]["message"]
        .as_str()
        .ok_or("missing root error message")?
        .contains("root_unreadable"));
    Ok(())
}

#[cfg(not(unix))]
#[test]
fn codegraph_explicit_unreadable_root_fixture_is_unavailable_on_non_unix() {
    eprintln!("unreadable-root fixture is unavailable on this operating system");
}

fn codegraph_stdio_indexes_searches_and_bounds_missing_neighbors(
    client: &mut McpClient,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let search_payload: Value = serde_json::from_str(search_text)?;
    let matches = search_payload["matches"].as_array().ok_or("search matches")?;
    assert_eq!(
        matches[0],
        json!("./entry.rs:2:pub const ENTRY: u8 = dep::VALUE;"),
        "codegraph_search must return the expected match"
    );
    assert_eq!(
        matches.len(),
        1,
        "codegraph_search must stop at the requested match limit"
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
    assert_eq!(neighbors["partial"], true);
    assert_eq!(neighbors["errors"][0]["kind"], "source_missing");
    assert_eq!(neighbors["imports"], json!([]));
    Ok(())
}

#[test]
fn codegraph_search_does_not_require_ambient_ripgrep() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("entry.rs"), "pub const ENTRY: u8 = 1;\n")?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"));
    command.env("PATH", "");
    let mut client = McpClient::spawn_command(command)?;
    let _init = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))?;

    let search = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{
            "root":root.path(),"query":"ENTRY","limit":10
        }}
    }))?;
    let search_text = search["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?;
    crate::support::assert_structured_literals(
        search_text,
        "codegraph search without ambient ripgrep",
        &["ENTRY"],
    );
    Ok(())
}

fn codegraph_stdio_matches_absolute_paths_when_root_is_relative(
    client: &mut McpClient,
    dependency: &Path,
    entry: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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
