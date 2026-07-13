use std::process::Command;

use tempfile::tempdir;

#[test]
fn rejects_boolean_wrong_version_and_duplicate_ids() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let cases = [
        (
            "{\"jsonrpc\":\"2.0\",\"id\":true,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "boolean",
        ),
        (
            "{\"jsonrpc\":\"1.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "version",
        ),
        (
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "duplicate",
        ),
    ];
    for (index, (payload, name)) in cases.into_iter().enumerate() {
        let file = root.path().join(format!("{index}.jsonl"));
        std::fs::write(&file, payload).expect("response fixture");
        let output = Command::new(&checker)
            .args([file.to_str().unwrap(), "test"])
            .output()
            .expect("checker");
        assert!(!output.status.success(), "{name} response should fail");
    }
}
