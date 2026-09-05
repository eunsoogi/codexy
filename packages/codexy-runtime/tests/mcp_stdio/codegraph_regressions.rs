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
    write_invalid_sources(repository.path())?;
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

#[test]
fn codegraph_stdio_bounds_search_errors_with_matches()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    write_invalid_sources(repository.path())?;
    std::fs::write(
        repository.path().join("zz_match.rs"),
        format!("MATCH {}\n", "x".repeat(120)),
    )?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;

    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{
            "root":repository.path(),"query":"MATCH","limit":80
        }}
    }))?;
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?;
    let payload: Value = serde_json::from_str(text)?;
    let matches = payload["matches"].as_array().ok_or("search matches")?;
    let errors = payload["errors"].as_array().ok_or("search errors")?;
    assert!(text.len() <= 8_192);
    assert!(!matches.is_empty());
    assert!(!errors.is_empty());
    assert_eq!(payload["partial"], true);
    assert_eq!(payload["truncation"]["contentBytes"], true);
    assert_eq!(payload["returnedMatchCount"], matches.len());
    Ok(())
}

#[test]
fn codegraph_stdio_refreshes_and_isolates_graphs_across_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let first = tempfile::tempdir()?;
    let first_dependency = first.path().join("dep.rs");
    let first_entry = first.path().join("entry.rs");
    std::fs::write(&first_dependency, "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(&first_entry, "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n")?;

    let second = tempfile::tempdir()?;
    let second_dependency = second.path().join("dep.rs");
    std::fs::write(&second_dependency, "pub const SECOND: u8 = 2;\n")?;
    std::fs::write(
        second.path().join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::SECOND;\n",
    )?;
    std::fs::create_dir(second.path().join(".git"))?;
    let ignored = second.path().join("ignored.rs");
    std::fs::write(&ignored, "pub const IGNORED: u8 = 3;\n")?;

    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        first.path(),
    )?;
    for id in 1..=10 {
        let response = client.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":"codegraph_reverse_deps","arguments":{
                "root":first.path(),"path":first_dependency,"limit":10
            }}
        }))?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("repeated reverse-deps text")?;
        let payload: Value = serde_json::from_str(text)?;
        assert_eq!(payload["dependents"][0]["path"], "entry.rs");
    }

    let changed_source = "mod xyz;\npub const ENTRY: u8 = xyz::VALUE;\n";
    assert_eq!(changed_source.len(), "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n".len());
    #[cfg(unix)]
    let first_entry_metadata = std::fs::metadata(&first_entry)?;
    std::fs::write(&first_entry, changed_source)?;
    #[cfg(unix)]
    restore_mtime(&first_entry, &first_entry_metadata)?;
    let changed = client.send(&json!({
        "jsonrpc":"2.0","id":11,"method":"tools/call",
        "params":{"name":"codegraph_reverse_deps","arguments":{
            "root":first.path(),"path":first_dependency,"limit":10
        }}
    }))?;
    let changed_text = changed["result"]["content"][0]["text"]
        .as_str()
        .ok_or("changed reverse-deps text")?;
    let changed_payload: Value = serde_json::from_str(changed_text)?;
    assert!(changed_payload["dependents"].as_array().is_some_and(Vec::is_empty));

    let second_index = client.send(&json!({
        "jsonrpc":"2.0","id":12,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":second.path()}}
    }))?;
    let second_text = second_index["result"]["content"][0]["text"]
        .as_str()
        .ok_or("second-root graph text")?;
    let second_payload: Value = serde_json::from_str(second_text)?;
    assert_eq!(second_payload["totalFiles"], 3);

    std::fs::write(second.path().join(".gitignore"), "ignored.rs\n")?;
    let ignored_index = client.send(&json!({
        "jsonrpc":"2.0","id":13,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":second.path()}}
    }))?;
    let ignored_text = ignored_index["result"]["content"][0]["text"]
        .as_str()
        .ok_or("ignore graph text")?;
    let ignored_payload: Value = serde_json::from_str(ignored_text)?;
    assert_eq!(ignored_payload["totalFiles"], 2);
    assert!(!ignored_payload["files"]
        .as_array()
        .ok_or("ignore graph files")?
        .iter()
        .any(|file| file == "ignored.rs"));

    std::fs::remove_file(second_dependency)?;
    let deleted_index = client.send(&json!({
        "jsonrpc":"2.0","id":14,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":second.path()}}
    }))?;
    let deleted_text = deleted_index["result"]["content"][0]["text"]
        .as_str()
        .ok_or("deleted graph text")?;
    let deleted_payload: Value = serde_json::from_str(deleted_text)?;
    assert_eq!(deleted_payload["totalFiles"], 1);
    let deleted_files = deleted_payload["files"]
        .as_array()
        .ok_or("deleted graph files")?;
    assert_eq!(deleted_files.len(), 1);
    assert_eq!(deleted_files[0]["path"], "entry.rs");
    Ok(())
}

fn write_invalid_sources(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    for index in 0..60 {
        let directory = root.join(format!("segment_{index:02}_{}", "x".repeat(120)));
        std::fs::create_dir(&directory)?;
        std::fs::write(
            directory.join(format!("bad_{index:02}_{}.rs", "y".repeat(80))),
            [0xff, 0xfe, b'\n'],
        )?;
    }
    Ok(())
}

#[cfg(unix)]
fn restore_mtime(
    path: &std::path::Path,
    metadata: &std::fs::Metadata,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;

    let path = CString::new(path.as_os_str().as_bytes())?;
    let times = [
        libc::timespec {
            tv_sec: metadata.atime(),
            tv_nsec: metadata.atime_nsec(),
        },
        libc::timespec {
            tv_sec: metadata.mtime(),
            tv_nsec: metadata.mtime_nsec(),
        },
    ];
    let result = unsafe {
        libc::utimensat(libc::AT_FDCWD, path.as_ptr(), times.as_ptr(), 0)
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
