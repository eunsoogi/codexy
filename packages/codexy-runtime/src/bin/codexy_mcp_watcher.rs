use std::io::{self, Read, Write};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

const MAX_HOOK_INPUT_BYTES: usize = 1_048_576;

fn main() -> Result<()> {
    match std::env::args().nth(1).as_deref() {
        None | Some("--stdio") => run_stdio(),
        Some("--hook-pretool") => run_hook(codexy_runtime::watcher::run_pretool_hook),
        Some("--hook-interrupt") => run_hook(codexy_runtime::watcher::run_interrupt_hook),
        Some(argument) => bail!("unsupported codexy-mcp-watcher argument: {argument}"),
    }
}

fn run_stdio() -> Result<()> {
    let tools = codexy_runtime::watcher::tools();
    codexy_runtime::mcp::run_stdio_server_with_cancellation(
        codexy_runtime::watcher::server_name(),
        codexy_runtime::version::runtime_version(),
        &tools,
        codexy_runtime::watcher::call_tool_with_cancellation,
    )
}

fn run_hook(handler: fn(&Value) -> Result<Value>) -> Result<()> {
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_HOOK_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("reading Watcher hook input")?;
    if bytes.len() > MAX_HOOK_INPUT_BYTES {
        bail!("Watcher hook input exceeds the size limit");
    }
    let payload: Value = serde_json::from_slice(&bytes).context("parsing Watcher hook input")?;
    let output = handler(&payload)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&serde_json::to_vec(&output)?)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
