use serde_json::Value;

#[test]
fn runtime_workflow_packages_release_artifacts_without_snapshot_branch()
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
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks",
        "gh release upload",
        "mkdir -p \"${plugin_root}/runtime\"",
        "cp dist/generated-runtimes/*.bin \"${plugin_root}/runtime/\"",
    ] {
        assert!(
            workflow.contains(required),
            "runtime workflow must package release artifacts; missing {required:?}"
        );
    }
    let package_validation_order = concat!(
        "--check-runtime-artifacts\n",
        "          scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks\n",
        "          tar -C"
    );
    assert!(
        workflow.contains(package_validation_order),
        "runtime workflow must validate hooks before creating the package archive"
    );
    for trigger in ["push:", "pull_request:"] {
        let trigger_text = workflow_trigger_block(&workflow, trigger)
            .ok_or_else(|| format!("runtime workflow missing {trigger}"))?;
        assert!(
            trigger_text.contains("plugins/codexy/hooks/**"),
            "runtime workflow {trigger} paths must include hooks"
        );
    }
    for forbidden in [
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH",
        "dist/marketplace-root",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "runtime workflow must not publish a generated marketplace branch; found {forbidden:?}"
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

fn workflow_trigger_block<'a>(workflow: &'a str, trigger: &str) -> Option<&'a str> {
    let start = workflow.find(trigger)?;
    let rest = &workflow[start..];
    let end = rest
        .match_indices("\n  ")
        .find_map(|(index, _)| {
            let next = &rest[index + 3..];
            (!next.starts_with(' ')).then_some(index)
        })
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

#[test]
fn release_contract_uses_main_for_current_marketplace_ref() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let publish: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let snapshot = publish["currentMarketplace"]
        .as_object()
        .ok_or("currentMarketplace object")?;
    let package = publish["package"].as_object().ok_or("package object")?;

    assert_eq!(
        publish["schema"],
        "codexy.internal.release-publish-contract.v1"
    );
    assert_eq!(snapshot["repository"], "https://github.com/eunsoogi/codexy");
    assert_eq!(snapshot["ref"], "main");
    assert_eq!(
        snapshot["marketplacePath"],
        ".agents/plugins/marketplace.json"
    );
    assert_eq!(snapshot["pluginPath"], "./plugins/codexy");
    assert_eq!(
        snapshot["installCommand"],
        "codex plugin marketplace add eunsoogi/codexy --ref main"
    );
    assert_eq!(
        package["workflow"],
        ".github/workflows/plugin-runtime-binaries.yml"
    );
    assert_eq!(package["futureInstallRef"], "version-tags");
    assert_eq!(
        package["platforms"],
        serde_json::json!(["darwin-arm64", "linux-x86_64"])
    );
    Ok(())
}
