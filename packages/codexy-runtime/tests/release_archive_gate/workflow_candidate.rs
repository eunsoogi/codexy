use serde_yaml::Value;

#[test]
// Candidate receipts follow the retained build/package inventory across all platforms.
fn candidate_workflow_builds_and_smokes_retained_runtimes_without_watcher() {
    let root = codexy_runtime::paths::repository_root();
    let source = std::fs::read_to_string(root.join(".github/workflows/runtime-candidate.yml"))
        .expect("candidate workflow");
    let workflow: Value = serde_yaml::from_str(&source).expect("candidate workflow YAML");
    for forbidden in ["codexy-mcp-watcher", "watcher_open", "coreWatcherMcp"] {
        assert!(
            !source.contains(forbidden),
            "candidate workflow contains {forbidden}"
        );
    }
    let platforms = workflow["jobs"]["build-runtime"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("candidate platform matrix");
    assert_eq!(
        platforms.len(),
        3,
        "candidate matrix must remain three-platform"
    );
    for platform in ["linux-x86_64", "darwin-arm64", "windows-x86_64"] {
        assert!(
            platforms.iter().any(|entry| entry["platform"] == platform),
            "candidate matrix omits {platform}"
        );
    }

    let steps = workflow["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .expect("candidate build steps");
    let step = |name: &str| {
        steps
            .iter()
            .find(|step| step["name"] == name)
            .unwrap_or_else(|| panic!("missing candidate step {name}"))
    };
    let build = step("Build candidate MCP runtimes")["run"]
        .as_str()
        .expect("candidate build command");
    for retained in [
        "--bin codexy-mcp-lsp",
        "--bin codexy-mcp-codegraph",
        "--bin codexy-mcp-devtools",
        "--bin codexy-handoff-validate",
    ] {
        assert!(build.contains(retained), "candidate build omits {retained}");
    }
    assert!(!build.contains("--bin codexy-mcp-watcher"));

    let unix_smoke = step("Smoke platform-local MCP protocols")["run"]
        .as_str()
        .expect("platform-local smoke");
    for retained in [
        "scripts/smoke_runtime_mcp.py",
        "lsp_status",
        "codegraph_overview",
    ] {
        assert!(
            unix_smoke.contains(retained),
            "platform smoke omits {retained}"
        );
    }
    let windows_smoke = step("Smoke native Windows MCP protocols")["run"]
        .as_str()
        .expect("Windows smoke");
    for retained in [
        "codexy-mcp-lsp.exe",
        "codexy-mcp-codegraph.exe",
        "tools/call",
    ] {
        assert!(
            windows_smoke.contains(retained),
            "Windows smoke omits {retained}"
        );
    }
    let handoff = step("Prove native handoff bridge and installed CMD adversarial contract")["run"]
        .as_str()
        .expect("Windows handoff checks");
    assert!(handoff.contains("handoff_runtime_bridge"));
    assert!(handoff.contains("tests.test_resumable_context_capsule"));

    for package_step in [
        step("Package declared platform binaries"),
        step("Package native Windows PE binaries"),
    ] {
        let package = package_step["run"]
            .as_str()
            .expect("platform package command");
        assert!(!package.contains("codexy-mcp-watcher"));
    }
}
