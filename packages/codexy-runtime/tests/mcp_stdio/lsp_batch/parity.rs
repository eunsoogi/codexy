use super::*;
use crate::system::mcp_stdio::fixture_gate::StderrPublicationGate;

#[test]
fn lsp_batch_rechecks_workspace_errors_after_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("sample.toml"), "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");

    let mut single_gate = StderrPublicationGate::new()?;
    let single_reader = single_gate.reader_address()?;
    let single_shutdown = single_gate.shutdown_address()?;
    let mut single = start_client(
        &[
            ("CODEXY_FAKE_LSP_STDERR", "FetchWorkspaceError: late failure"),
            ("CODEXY_TEST_STDERR_GATE_ADDR", single_reader.as_str()),
            (
                "CODEXY_TEST_STDERR_SHUTDOWN_GATE_ADDR",
                single_shutdown.as_str(),
            ),
        ],
        None,
    )?;
    initialize(&mut single)?;
    let single_response = single.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_document_symbols","arguments":{
            "root":root.path(),"path":"sample.toml",
            "server":{"id":"taplo","command":[fake_lsp]},"timeoutMs":5000
        }}
    }))?;
    single_gate.complete()?;
    let single_payload = text_payload(&single_response)?;
    assert_eq!(single_payload["status"], "error");
    assert!(single_payload["reason"]
        .as_str()
        .ok_or("single workspace error reason")?
        .contains("FetchWorkspaceError"));

    let mut batch_gate = StderrPublicationGate::new()?;
    let batch_reader = batch_gate.reader_address()?;
    let batch_shutdown = batch_gate.shutdown_address()?;
    let mut batch = start_client(
        &[
            ("CODEXY_FAKE_LSP_STDERR", "FetchWorkspaceError: late failure"),
            ("CODEXY_TEST_STDERR_GATE_ADDR", batch_reader.as_str()),
            (
                "CODEXY_TEST_STDERR_SHUTDOWN_GATE_ADDR",
                batch_shutdown.as_str(),
            ),
        ],
        None,
    )?;
    initialize(&mut batch)?;
    let batch_response = batch_response(
        &mut batch,
        2,
        batch_arguments(
            root.path(),
            fake_lsp,
            json!([{"method":"lsp_document_symbols","path":"sample.toml"}]),
        ),
    )?;
    batch_gate.complete()?;
    let batch_payload = text_payload(&batch_response)?;
    assert_eq!(batch_payload["status"], single_payload["status"]);
    assert_eq!(batch_payload["results"][0]["status"], "error");
    assert!(batch_payload["results"][0]["reason"]
        .as_str()
        .ok_or("batch workspace error reason")?
        .contains("FetchWorkspaceError"));
    Ok(())
}

#[test]
fn lsp_single_request_reads_the_file_before_server_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let mut client = start_client(
        &[("CODEXY_FAKE_LSP_RESPONSE_ERROR", "fixture init failure")],
        None,
    )?;
    initialize(&mut client)?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_document_symbols","arguments":{
            "root":root.path(),"path":"missing.toml",
            "server":{"id":"taplo","command":[fake_lsp]},"timeoutMs":5000
        }}
    }))?;
    let payload = text_payload(&response)?;
    let reason = payload["reason"].as_str().ok_or("missing-file reason")?;
    assert!(reason.contains("reading"), "{payload:#}");
    assert!(reason.contains("missing.toml"), "{payload:#}");
    Ok(())
}

#[test]
fn lsp_single_request_preserves_push_diagnostics_timeout_behavior() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("sample.toml"), "value = 1\n")?;
    let mut client = start_client(&[("CODEXY_FAKE_LSP_NO_PULL_DIAGNOSTICS", "1")], None)?;
    initialize(&mut client)?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{
            "root":root.path(),"path":"sample.toml",
            "server":{"id":"taplo","command":[env!("CARGO_BIN_EXE_codexy-fake-lsp")]},"timeoutMs":1000
        }}
    }))?;
    let payload = text_payload(&response)?;
    assert_eq!(payload["status"], "ok", "{payload:#}");
    Ok(())
}
