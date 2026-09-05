use super::*;

#[test]
fn codegraph_stdio_rejects_an_explicit_missing_root_in_the_mcp_error_envelope()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let missing = repository.path().join("missing-repository");
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":missing}}
    }))?;

    assert_eq!(response["error"]["code"], -32000);
    let message = response["error"]["message"]
        .as_str()
        .ok_or("missing MCP error message")?;
    assert!(message.contains("root_missing"), "unexpected error: {message}");
    assert!(message.contains("missing-repository"), "unexpected error: {message}");
    assert!(response.get("result").is_none());
    Ok(())
}

#[test]
fn codegraph_stdio_reports_source_encoding_errors_as_partial_results()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    std::fs::write(repository.path().join("bad.rs"), [0xff, 0xfe, b'\n'])?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":repository.path()}}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("graph text")?;
    let graph: Value = serde_json::from_str(text)?;

    assert_eq!(graph["partial"], true);
    assert_eq!(graph["errors"][0]["kind"], "encoding_failure");
    assert_eq!(graph["errors"][0]["path"], "bad.rs");
    assert!(graph["files"]
        .as_array()
        .ok_or("graph files")?
        .iter()
        .any(|file| file["path"] == "bad.rs"));
    Ok(())
}

#[test]
fn codegraph_stdio_attaches_partial_errors_to_each_source_reading_tool()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    std::fs::write(repository.path().join("bad.rs"), [0xff, 0xfe, b'\n'])?;
    let requests = [
        ("codegraph_overview", json!({"root":repository.path()})),
        (
            "codegraph_search",
            json!({"root":repository.path(),"query":"ENTRY"}),
        ),
        ("codegraph_index", json!({"root":repository.path()})),
        (
            "codegraph_reverse_deps",
            json!({"root":repository.path(),"path":"bad.rs"}),
        ),
        (
            "codegraph_neighborhood",
            json!({"root":repository.path(),"path":"bad.rs"}),
        ),
        (
            "codegraph_neighbors",
            json!({"root":repository.path(),"path":"bad.rs"}),
        ),
    ];
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    for (id, (name, arguments)) in requests.into_iter().enumerate() {
        let response = client.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}
        }))?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("partial result text")?;
        let payload: Value = serde_json::from_str(text)?;
        assert_eq!(payload["partial"], true, "{name} should be partial");
        assert_eq!(payload["errors"][0]["kind"], "encoding_failure");
    }
    Ok(())
}

#[test]
fn codegraph_stdio_reports_source_missing_after_a_prior_traversal()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let source = repository.path().join("deleted.rs");
    std::fs::write(&source, "pub const DELETED: u8 = 1;\n")?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let _walk = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_overview","arguments":{"root":repository.path()}}
    }))?;
    std::fs::remove_file(&source)?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_neighbors","arguments":{"root":repository.path(),"path":"deleted.rs"}}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("deletion result text")?;
    let payload: Value = serde_json::from_str(text)?;
    assert_eq!(payload["errors"][0]["kind"], "source_missing");
    assert!(!source.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn codegraph_stdio_reports_permission_errors_for_unreadable_sources()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let repository = tempfile::tempdir()?;
    let source = repository.path().join("blocked.rs");
    std::fs::write(&source, "pub const BLOCKED: u8 = 1;\n")?;
    let mut permissions = std::fs::metadata(&source)?.permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&source, permissions)?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":repository.path()}}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("permission result text")?;
    let payload: Value = serde_json::from_str(text)?;
    if payload["errors"].is_null() {
        eprintln!("permission denial is unavailable for the current privileged test user");
        return Ok(());
    }
    assert_eq!(payload["errors"][0]["kind"], "permission_denied");
    Ok(())
}

#[cfg(not(unix))]
#[test]
fn codegraph_permission_fixture_is_unavailable_on_non_unix() {
    eprintln!("permission denial fixture is unavailable on this operating system");
}

pub(super) fn codegraph_stdio_keeps_outside_absolute_paths_distinct(
    client: &mut McpClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let outside_dep = outside.path().join("dep.rs");
    std::fs::write(&outside_dep, "pub const OUTSIDE: u8 = 1;\n")?;
    let canonical_outside = outside_dep.canonicalize()?;
    let mirrored_suffix = canonical_outside
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect::<std::path::PathBuf>();
    let mirrored_dep = root.path().join(mirrored_suffix);
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
