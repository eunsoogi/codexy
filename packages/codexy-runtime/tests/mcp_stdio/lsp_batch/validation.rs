use super::*;

#[test]
fn lsp_batch_rejects_bounds_context_and_path_escape_before_spawn() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir_in(root.path().parent().ok_or("root parent")?)?;
    std::fs::write(outside.path().join("outside.toml"), "value = 1\n")?;
    let outside_name = outside
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("outside directory name")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let capture = root.path().join("should-not-exist.json");
    let mut client = start_client(&[], Some(&capture))?;
    initialize(&mut client)?;
    let nine = Value::Array(
        (0..9)
            .map(|_| json!({"method":"lsp_document_symbols","path":"x.toml"}))
            .collect(),
    );
    let response = batch_response(&mut client, 2, batch_arguments(root.path(), fake_lsp, nine))?;
    assert!(response["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("between 1 and 8"));
    let mut deadline = batch_arguments(
        root.path(),
        fake_lsp,
        json!([{"method":"lsp_document_symbols","path":"x.toml"}]),
    );
    deadline["deadlineMs"] = json!(60001);
    let response = batch_response(&mut client, 3, deadline)?;
    assert!(response["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("deadlineMs"));
    let mut item_context = batch_arguments(
        root.path(),
        fake_lsp,
        json!([{"method":"lsp_document_symbols","path":"x.toml","root":root.path()}]),
    );
    item_context["requests"][0]["root"] = json!(root.path());
    let response = batch_response(&mut client, 4, item_context)?;
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("once at batch level"),
        "{response:#}"
    );
    let escaped = batch_arguments(
        root.path(),
        fake_lsp,
        json!([{"method":"lsp_document_symbols","path":PathBuf::from("..").join(outside_name).join("outside.toml")}]),
    );
    let response = batch_response(&mut client, 5, escaped)?;
    assert!(response["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("escapes root"), "{response:#}");
    assert!(!capture.exists());
    Ok(())
}

#[test]
fn lsp_batch_rejects_inferred_server_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("sample.toml"), "value = 1\n")?;
    std::fs::write(root.path().join("sample.rs"), "fn main() {}\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let mut client = start_client(&[], None)?;
    initialize(&mut client)?;
    let mut arguments = batch_arguments(
        root.path(),
        fake_lsp,
        json!([
            {"method":"lsp_document_symbols","path":"sample.toml"},
            {"method":"lsp_document_symbols","path":"sample.rs"}
        ]),
    );
    arguments
        .as_object_mut()
        .ok_or("batch arguments")?
        .remove("server");
    let payload = text_payload(&batch_response(&mut client, 2, arguments)?)?;
    assert_eq!(payload["status"], "error");
    assert_eq!(
        payload["reason"],
        "lsp_batch requests must resolve to one server"
    );
    Ok(())
}
