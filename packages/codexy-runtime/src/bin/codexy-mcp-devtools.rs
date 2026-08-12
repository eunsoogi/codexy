use std::{
    env,
    path::PathBuf,
    process::{Command, exit},
};

use anyhow::{Context as _, Result, bail};

fn main() -> Result<()> {
    let mut arguments = env::args_os();
    let executable = arguments
        .next()
        .context("missing codexy-mcp-devtools executable path")?;
    let server = arguments
        .next()
        .context("codexy-mcp-devtools migration: expected lsp or codegraph")?;
    let server = server
        .to_str()
        .filter(|server| matches!(*server, "lsp" | "codegraph"))
        .context("codexy-mcp-devtools migration: expected lsp or codegraph")?;
    let runtime = runtime_path(PathBuf::from(executable), server)?;
    let status = Command::new(&runtime)
        .args(arguments)
        .status()
        .with_context(|| format!("starting {}", runtime.display()))?;
    if let Some(code) = status.code() {
        exit(code);
    }
    bail!("{} exited without a status", runtime.display())
}

fn runtime_path(executable: PathBuf, server: &str) -> Result<PathBuf> {
    let mcp = executable
        .parent()
        .context("codexy-mcp-devtools executable has no parent directory")?;
    let plugin = mcp
        .parent()
        .context("codexy-mcp-devtools executable must live in mcp/")?;
    Ok(plugin
        .join("runtime")
        .join(format!("codexy-mcp-{server}-windows-x86_64.exe")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::runtime_path;

    #[test]
    fn windows_dispatcher_resolves_the_single_server_runtime() {
        let runtime = runtime_path(
            PathBuf::from("C:/plugin/mcp/codexy-mcp-devtools.exe"),
            "lsp",
        )
        .expect("valid launcher path");
        assert_eq!(
            runtime,
            PathBuf::from("C:/plugin/runtime/codexy-mcp-lsp-windows-x86_64.exe")
        );
    }
}
