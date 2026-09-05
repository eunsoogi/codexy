use super::*;
use std::process::Command;
use std::time::Instant;

#[test]
fn lsp_batch_matches_single_result_with_installed_rust_analyzer()
-> Result<(), Box<dyn std::error::Error>> {
    if !Command::new("rust-analyzer")
        .arg("--version")
        .status()?
        .success()
    {
        eprintln!("rust-analyzer is unavailable; skipping installed-server comparison");
        return Ok(());
    }
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("src"))?;
    std::fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"lsp-batch-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    std::fs::write(root.path().join("src/lib.rs"), "pub fn fixture() -> u32 { 1 }\n")?;
    let server = json!({"id":"rust-analyzer","command":["rust-analyzer"]});
    let mut client = start_client(&[], None)?;
    initialize(&mut client)?;
    let single_start = Instant::now();
    let single = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_document_symbols","arguments":{
            "root":root.path(),"workspaceRoot":root.path(),"path":"src/lib.rs",
            "server":server,"timeoutMs":60000
        }}
    }))?;
    let single_elapsed = single_start.elapsed();
    let single_payload = text_payload(&single)?;
    assert_eq!(single_payload["status"], "ok");
    let batch_start = Instant::now();
    let batch = batch_response(
        &mut client,
        3,
        json!({
            "root":root.path(),"workspaceRoot":root.path(),"server":server,
            "timeoutMs":60000,"deadlineMs":60000,
            "requests":[{"method":"lsp_document_symbols","path":"src/lib.rs"}]
        }),
    )?;
    let batch_elapsed = batch_start.elapsed();
    let batch_payload = text_payload(&batch)?;
    assert_eq!(batch_payload["status"], "ok");
    assert_eq!(batch_payload["results"][0]["result"], single_payload["result"]);
    eprintln!(
        "installed rust-analyzer latency: single_ms={} batch_ms={}",
        single_elapsed.as_millis(),
        batch_elapsed.as_millis()
    );
    Ok(())
}
