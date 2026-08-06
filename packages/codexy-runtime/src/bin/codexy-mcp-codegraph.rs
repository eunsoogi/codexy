use anyhow::Result;

fn main() -> Result<()> {
    let tools = codexy_runtime::codegraph::tools::tools();
    codexy_runtime::mcp::run_stdio_server(
        codexy_runtime::codegraph::server_name(),
        env!("CARGO_PKG_VERSION"),
        &tools,
        codexy_runtime::codegraph::tools::call_tool,
    )
}
