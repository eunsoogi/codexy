use anyhow::Result;

fn main() -> Result<()> {
    let tools = codexy_runtime::lsp::tools();
    codexy_runtime::mcp::run_stdio_server(
        codexy_runtime::lsp::server_name(),
        env!("CARGO_PKG_VERSION"),
        &tools,
        codexy_runtime::lsp::call_tool,
    )
}
