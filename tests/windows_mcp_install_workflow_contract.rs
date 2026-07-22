use std::path::Path;

#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;

use structured_contract_artifacts::TextShape;

#[test]
fn windows_install_proves_direct_pe_and_codex_configured_launch() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "windows-installed-mcp-evidence",
        &[
            "$entry.transport.command -ne \"./mcp/codexy-mcp-$server\"",
            "Get-FileHash $runtime",
            "function Test-PathWithin",
            "[System.IO.Path]::GetRelativePath",
            "$env:CODEX_HOME + \"-sibling\"",
            "Label = \"installed PE\"",
            "$env:PATHEXT = \".EXE\"",
            "%~dp0cmd-sentinel-ran",
            "$start.ArgumentList.Add(\"app-server\")",
            "$appServer.StandardInput.WriteLine('{\"method\":\"initialized\"}')",
            "mcpServerStatus/list",
            "$statusResponseTimeout = [TimeSpan]::FromSeconds(30)",
            "ReadLineAsync().WaitAsync($statusResponseTimeout)",
            "detail = \"toolsAndAuthOnly\"",
            "$appServerFailure = $_.Exception.Message",
            "Codex app-server stderr: $appServerStderr",
            "installed MCP process cleanup failed",
        ],
    );
    let shape = TextShape::new(&workflow);
    shape.assert_absent_concepts(
        "windows launch proof does not use a PowerShell resolution surrogate",
        &[
            "Label configured extensionless command",
            "Get Command Name $entrypoint Path",
        ],
    );
    shape.assert_absent_concepts(
        "windows path containment compares components",
        &[
            "$installedPath StartsWith",
            "$entry transport cwd StartsWith",
            "$_ Path StartsWith",
        ],
    );
}
