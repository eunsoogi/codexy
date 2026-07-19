use std::process::Command;

use serde_json::Value;

#[test]
fn runtime_workflow_packages_release_artifacts_without_snapshot_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;

    for required in [
        "package-plugin:",
        "needs: build-runtime",
        "actions/download-artifact@",
        "pattern: codexy-mcp-runtimes-*",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-runtime-artifacts",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks",
        "gh release upload",
        "gh release upload \"$RELEASE_TAG\" \"dist/codexy-marketplace-plugin.tar.gz\" --clobber",
        "mkdir -p \"${plugin_root}/runtime\"",
        "cp dist/generated-runtimes/*.bin \"${plugin_root}/runtime/\"",
        "push:",
        "tags:",
        "\"v*\"",
        "Generate commit-log changelog",
        "git rev-list -n 1 \"$release_tag\"",
        "scripts/generate-release-changelog \"$release_tag\" \"$PREVIOUS_TAG\" > release-notes.md",
        "Create or update GitHub release",
        "--target \"$RELEASE_TARGET\"",
        "gh release create \"$release_tag\"",
        "gh release edit \"$release_tag\"",
        "needs: [build-runtime-tool, verify-release-source, publish-release]",
    ] {
        // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
        assert!(
            workflow.find(required).is_some(),
            "runtime workflow must package release artifacts; missing {required:?}"
        );
    }
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(
        workflow.find("--target \"$GITHUB_SHA\"").is_none(),
        "manual release workflow must target the commit behind release_tag, not the workflow ref"
    );
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("git merge-base --is-ancestor \"$GITHUB_SHA\" origin/main").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("if: startsWith(github.ref, 'refs/tags/')").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find(
        "if: github.event_name == 'release' || startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch'"
    ).is_none());
    let package_validation_order = concat!(
        "--check-runtime-artifacts\n",
        "          scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks\n",
        "          tar -C"
    );
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(
        workflow.find(package_validation_order).is_some(),
        "runtime workflow must validate hooks before creating the package archive"
    );
    for trigger in ["push:", "pull_request:"] {
        let trigger_text = workflow_trigger_block(&workflow, trigger)
            .ok_or_else(|| format!("runtime workflow missing {trigger}"))?;
        for required_path in [
            "plugins/codexy/**",
            "scripts/inspect-mcp-response",
            "scripts/generate-release-changelog",
        ] {
            // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
            assert!(
                trigger_text.find(required_path).is_some(),
                "runtime workflow {trigger} paths must include {required_path}"
            );
        }

        for packaged_source in [
            "plugins/codexy/**",
            ".agents/plugins/marketplace.json",
            ".agents/plugins/release-publish-contract.json",
        ] {
            // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
            assert!(
                trigger_text.find(packaged_source).is_some(),
                "runtime workflow {trigger} paths must cover packaged source inventory entry {packaged_source}"
            );
        }
        // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
        assert!(
            trigger_text.find("README.md").is_none() && trigger_text.find("tests/**").is_none(),
            "runtime workflow {trigger} paths must not include unrelated repository paths"
        );
    }
    for forbidden in [
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH",
        "dist/marketplace-root",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
    ] {
        // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
        assert!(
            workflow.find(forbidden).is_none(),
            "runtime workflow must not publish a generated marketplace branch; found {forbidden:?}"
        );
    }
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(
        workflow.find("plugins/codexy/bin").is_none()
            && workflow.find("${plugin_root}/bin").is_none()
            && workflow.find("\"$plugin_root\"/bin").is_none(),
        "runtime workflow must not use plugin bin paths as its install contract"
    );
    Ok(())
}

#[test]
fn release_changelog_script_formats_single_commit_range() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;

    run_git(temp.path(), &["init"])?;
    run_git(temp.path(), &["config", "user.email", "codexy@example.com"])?;
    run_git(temp.path(), &["config", "user.name", "Codexy Test"])?;
    std::fs::write(temp.path().join("file.txt"), "before\n")?;
    run_git(temp.path(), &["add", "file.txt"])?;
    run_git(temp.path(), &["commit", "-m", "before release"])?;
    run_git(temp.path(), &["tag", "v0.1.0"])?;
    std::fs::write(temp.path().join("file.txt"), "after\n")?;
    run_git(temp.path(), &["add", "file.txt"])?;
    run_git(temp.path(), &["commit", "-m", "one change"])?;
    run_git(temp.path(), &["tag", "v0.2.0"])?;

    let output = Command::new(root.join("scripts/generate-release-changelog"))
        .current_dir(temp.path())
        .args(["v0.2.0", "v0.1.0"])
        .output()?;

    assert!(
        output.status.success(),
        "script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Changes since v0.1.0:"));
    assert!(stdout.contains("- one change ("));
    assert!(
        !stdout.contains("No commits found"),
        "one-commit range must not use empty-changelog fallback:\n{stdout}"
    );
    assert!(
        stdout.ends_with('\n'),
        "changelog output should end with a newline:\n{stdout}"
    );
    Ok(())
}

fn run_git(cwd: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
fn touched_loc_workflow_runs_for_all_pull_requests() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/touched-loc-gate.yml"))?;

    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("pull_request:").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(
        workflow.find("paths:").is_none(),
        "touched LOC gate must not use a narrow paths filter"
    );
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("fetch-depth: 0").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("--check-touched-loc").is_some());
    Ok(())
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
