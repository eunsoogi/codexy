pub(crate) fn assert_structured_literals(text: &str, rule_id: &str, required: &[&str]) {
    let missing: Vec<_> = required
        .iter()
        .filter(|literal| !text.contains(**literal))
        .collect();
    assert!(
        missing.is_empty(),
        "structured contract {rule_id} is missing required literals {missing:?}"
    );
}

#[allow(dead_code)]
pub(crate) fn assert_archive_scanner_contract(script: &str, checker: &str) {
    assert_structured_literals(
        script,
        "archive scanner behavior",
        &[
            "rg -a -n",
            "grep -a -Hn",
            "!**/runtime/*",
            "!**/mcp/*.exe",
            "! -name '*.md'",
            "! -name '*.txt'",
            "command -v python3",
            "rg or grep is required",
            "hygiene scan failed",
            "duplicate archive entries",
            "unexpected runtime artifact",
            "unsafe archive path",
        ],
    );
    assert_structured_literals(
        checker,
        "MCP response checker behavior",
        &[
            "invalid JSON-RPC version for response id",
            "set(responses) != {1, 2}",
        ],
    );
}

#[allow(dead_code)]
pub(crate) fn assert_runtime_workflow_contract(workflow: &str) {
    assert_structured_literals(
        workflow,
        "runtime workflow coverage",
        &[
            "scripts/validate-plugin-config --plugin-root plugins/codexy --check\n          rsync -a",
            "Smoke test native POSIX MCP runtimes",
            "Smoke test native Windows MCP runtimes",
            "Verify clean native Windows plugin MCP install",
            "$archivePath = (Resolve-Path -LiteralPath \"dist/codexy-marketplace-plugin.tar.gz\").Path",
            "Push-Location -LiteralPath $marketplaceRoot",
            "mcpServerStatus/list",
            "needs: [package-plugin, windows-installed-mcp]",
        ],
    );
}
