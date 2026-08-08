#[test]
fn lsp_server_name_preserves_legacy_mcp_contract() {
    // Given: callers assert the MCP initialize serverInfo name from the legacy JS server.
    // When: the Rust LSP MCP binary reports its server name.
    let name = codexy_runtime::lsp::server_name();

    // Then: the name remains stable across the runtime rewrite.
    assert_eq!(name, "codexy-lsp");
}
