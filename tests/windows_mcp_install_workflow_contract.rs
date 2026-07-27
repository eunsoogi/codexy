use std::path::Path;

#[test]
fn windows_selected_candidate_proof_preserves_legacy_public_boundary() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "windows-selected-candidate-proof",
        &[
            "verify-windows-selected-candidate:",
            "Verify immutable native Windows candidate bytes",
            "legacy-public baseline intentionally has no selected Windows candidate",
            "candidate-proven",
            "Get-FileHash -Algorithm SHA256 $archive",
            "System32/tar.exe",
            "codexy-mcp-$server-windows-x86_64.exe",
            "codexy-mcp-$server.exe",
            "$server entrypoint differs from its runtime",
        ],
    );
}
