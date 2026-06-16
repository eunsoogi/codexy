use anyhow::Result;

fn main() -> Result<()> {
    let tools = codexy_runtime::codegraph::tools::tools();
    codexy_runtime::mcp::run_stdio_server(
        codexy_runtime::codegraph::server_name(),
        "0.1.0",
        &tools,
        codexy_runtime::codegraph::tools::call_tool,
    )
}
