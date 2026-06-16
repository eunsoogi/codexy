use anyhow::Result;

fn main() -> Result<()> {
    let tools = codexy_runtime::lsp::tools();
    codexy_runtime::mcp::run_stdio_server(
        codexy_runtime::lsp::server_name(),
        "0.1.0",
        &tools,
        codexy_runtime::lsp::call_tool,
    )
}
