use serde_json::Value;

#[test]
fn runtime_workflow_publishes_installable_marketplace_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;

    for required in [
        "release:",
        "package-plugin:",
        "needs: build-runtime",
        "actions/download-artifact@v4",
        "pattern: codexy-mcp-runtimes-*",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-runtime-artifacts",
        "gh release upload",
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH: codexy-marketplace",
        "dist/marketplace-root",
        "cp .agents/plugins/marketplace.json \"$marketplace_root/.agents/plugins/marketplace.json\"",
        "cp -R \"$PACKAGE_ROOT/plugins/codexy\" \"$marketplace_root/plugins/codexy\"",
        "scripts/validate-plugin-config --plugin-root \"$marketplace_root/plugins/codexy\" --check-runtime-artifacts",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
        "mkdir -p \"${plugin_root}/runtime\"",
        "cp dist/generated-runtimes/*.bin \"${plugin_root}/runtime/\"",
    ] {
        assert!(
            workflow.contains(required),
            "runtime workflow must publish a generated marketplace snapshot; missing {required:?}"
        );
    }
    assert!(
        !workflow.contains("plugins/codexy/bin")
            && !workflow.contains("${plugin_root}/bin")
            && !workflow.contains("\"$plugin_root\"/bin"),
        "runtime workflow must not use plugin bin paths as its install contract"
    );
    Ok(())
}

#[test]
fn release_contract_names_generated_marketplace_ref() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let publish: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let snapshot = publish["marketplaceSnapshot"]
        .as_object()
        .ok_or("marketplaceSnapshot object")?;
    let package = publish["package"].as_object().ok_or("package object")?;

    assert_eq!(
        publish["schema"],
        "codexy.internal.release-publish-contract.v1"
    );
    assert_eq!(snapshot["repository"], "https://github.com/eunsoogi/codexy");
    assert_eq!(snapshot["ref"], "codexy-marketplace");
    assert_eq!(
        snapshot["marketplacePath"],
        ".agents/plugins/marketplace.json"
    );
    assert_eq!(snapshot["pluginPath"], "./plugins/codexy");
    assert_eq!(
        snapshot["installCommand"],
        "codex plugin marketplace add eunsoogi/codexy --ref codexy-marketplace"
    );
    assert_eq!(
        package["workflow"],
        ".github/workflows/plugin-runtime-binaries.yml"
    );
    assert_eq!(
        package["platforms"],
        serde_json::json!(["darwin-arm64", "linux-x86_64"])
    );
    Ok(())
}
