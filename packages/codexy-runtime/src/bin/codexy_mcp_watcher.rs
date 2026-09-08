use anyhow::Result;

fn main() -> Result<()> {
    let tools = codexy_runtime::watcher::tools();
    codexy_runtime::mcp::run_stdio_server_with_cancellation(
        codexy_runtime::watcher::server_name(),
        codexy_runtime::version::runtime_version(),
        &tools,
        codexy_runtime::watcher::call_tool_with_cancellation,
    )
}
