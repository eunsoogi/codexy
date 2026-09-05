use super::*;

#[test]
fn codegraph_stdio_neighbors_accepts_absolute_source_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    std::fs::write(repository.path().join("dep.rs"), "pub const VALUE: u8 = 1;\n")?;
    let entry = repository.path().join("entry.rs");
    std::fs::write(&entry, "use dep::VALUE;\npub const ENTRY: u8 = VALUE;\n")?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;

    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_neighbors","arguments":{
            "root":repository.path(),"path":entry
        }}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("neighbors text")?;
    let payload: Value = serde_json::from_str(text)?;
    let imports = payload.as_array().ok_or("absolute path must preserve success shape")?;
    assert!(imports.iter().any(|line| line["text"] == "use dep::VALUE;"));
    Ok(())
}

#[test]
fn codegraph_stdio_bounds_serialized_search_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    for index in 0..60 {
        let directory = repository
            .path()
            .join(format!("segment_{index:02}_{}", "x".repeat(120)));
        std::fs::create_dir(&directory)?;
        std::fs::write(
            directory.join(format!("bad_{index:02}_{}.rs", "y".repeat(80))),
            [0xff, 0xfe, b'\n'],
        )?;
    }
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;

    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{
            "root":repository.path(),"query":".","limit":80
        }}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?;
    let payload: Value = serde_json::from_str(text)?;
    assert!(
        text.len() <= 8_192,
        "search payload exceeded content budget: {} bytes",
        text.len()
    );
    assert_eq!(payload["partial"], true);
    assert_eq!(payload["truncation"]["contentBytes"], true);
    assert!(!payload["errors"].as_array().ok_or("search errors")?.is_empty());
    Ok(())
}
