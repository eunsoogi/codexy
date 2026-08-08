use super::*;

#[test]
fn archive_header_receipt_records_the_raw_governed_wrapper_header() {
    let root = tempfile::tempdir().expect("tempdir");
    let archive = root.path().join("fixture.tar");
    let receipt = root.path().join("headers.jsonl");
    let wrapper = "plugins/codexy/mcp/codexy-mcp-lsp".to_owned();
    let mut bytes = vec![0_u8; 512 * 3];
    bytes[..wrapper.len()].copy_from_slice(wrapper.as_bytes());
    bytes[100..108].copy_from_slice(b"0000644\0");
    bytes[124..136].copy_from_slice(b"00000000001\0");
    bytes[512] = b'x';
    std::fs::write(&archive, bytes).expect("archive");

    archive_evidence::append_archive_header_receipt(&receipt, "before", &archive, &[wrapper])
        .expect("receipt");

    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).expect("receipt bytes"))
            .expect("receipt JSON");
    assert_eq!(receipt["phase"], "before");
    assert_eq!(receipt["missingWrappers"], serde_json::json!([]));
    assert_eq!(receipt["headers"][0]["mode"], "3030303036343400");
    assert_eq!(
        receipt["headers"][0]["header_hex"].as_str().map(str::len),
        Some(1024)
    );
}
