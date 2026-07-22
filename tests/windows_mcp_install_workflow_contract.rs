use std::path::Path;

#[test]
fn windows_install_proves_the_configured_command_and_retains_failure_evidence() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "windows-installed-mcp-evidence",
        &[
            "Label = \"installed PE\"",
            "Label = \"configured extensionless command\"",
            "Path = Join-Path $installedPath \"mcp/codexy-mcp-$server\"",
            "$env:PATHEXT = \".EXE\"",
            "Get-Command -Name $entrypoint.Path -CommandType Application",
            "[string]::Equals($resolvedCommand.Path, $expectedPePath",
            "$entrypointPath = $resolvedCommand.Path",
            "%~dp0cmd-sentinel-ran",
            "$appServerFailure = $_.Exception.Message",
            "Codex app-server stderr: $appServerStderr",
            "installed MCP process cleanup failed",
        ],
    );
}
