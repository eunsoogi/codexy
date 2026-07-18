use std::process::Command;

use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

mod support;

use support::workflow_contract::{
    mapping_field as yaml_mapping_field, step as yaml_step, steps as yaml_steps,
    string_field as yaml_string_field, string_sequence as yaml_string_sequence,
};

#[test]
fn runtime_workflow_packages_release_artifacts_without_snapshot_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;
    let document: YamlValue = serde_yaml::from_str(&workflow)?;
    let workflow = document.as_mapping().ok_or("workflow root")?;
    let jobs = yaml_mapping_field(workflow, "jobs", "workflow")?;
    let package = yaml_mapping_field(jobs, "package-plugin", "jobs")?;
    assert_eq!(yaml_string_field(package, "needs")?, "build-runtime");
    let publish = yaml_mapping_field(jobs, "publish-release", "jobs")?;
    assert_eq!(
        yaml_string_sequence(publish.get("needs").ok_or("publish needs")?)?,
        ["package-plugin", "windows-installed-mcp"]
    );

    let package_steps = yaml_steps(package)?;
    let download = yaml_step(package_steps, "Download generated runtime binaries")?;
    assert_eq!(
        yaml_string_field(download, "uses")?,
        "actions/download-artifact@v4"
    );
    let download_inputs = yaml_mapping_field(download, "with", "download step")?;
    assert_eq!(
        yaml_string_field(download_inputs, "pattern")?,
        "codexy-mcp-runtimes-*"
    );
    let assemble = yaml_step(package_steps, "Assemble marketplace plugin package")?;
    let assemble_run = yaml_string_field(assemble, "run")?;
    let assemble_lines: Vec<_> = assemble_run.lines().map(str::trim).collect();
    for required in [
        "mkdir -p \"${plugin_root}/runtime\"",
        "cp dist/generated-runtimes/* \"${plugin_root}/runtime/\"",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-runtime-artifacts",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks",
        "tar -C \"$package_root\" -czf \"dist/codexy-marketplace-plugin.tar.gz\" plugins/codexy",
    ] {
        assert!(
            assemble_lines.iter().any(|line| *line == required),
            "missing package command {required:?}"
        );
    }
    let runtime_validation = assemble_lines
        .iter()
        .position(|line| *line == "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-runtime-artifacts")
        .ok_or("runtime validation command")?;
    let hook_validation = assemble_lines
        .iter()
        .position(|line| {
            *line == "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-hooks"
        })
        .ok_or("hook validation command")?;
    let archive = assemble_lines
        .iter()
        .position(|line| *line == "tar -C \"$package_root\" -czf \"dist/codexy-marketplace-plugin.tar.gz\" plugins/codexy")
        .ok_or("archive command")?;
    assert!(runtime_validation < hook_validation && hook_validation < archive);

    let triggers = workflow
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == YamlValue::Bool(true))
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("workflow triggers")?;
    for trigger in ["push", "pull_request"] {
        let trigger = yaml_mapping_field(triggers, trigger, "workflow triggers")?;
        let paths = yaml_string_sequence(trigger.get("paths").ok_or("trigger paths")?)?;
        for required in [
            "plugins/codexy/**",
            "scripts/inspect-mcp-response",
            "scripts/generate-release-changelog",
            ".agents/plugins/marketplace.json",
            ".agents/plugins/release-publish-contract.json",
        ] {
            assert!(
                paths.iter().any(|path| *path == required),
                "missing trigger path {required}"
            );
        }
        assert!(
            paths
                .iter()
                .all(|path| *path != "README.md" && *path != "tests/**")
        );
    }

    let publish_steps = yaml_steps(publish)?;
    let changelog = yaml_step(publish_steps, "Generate commit-log changelog")?;
    let changelog_lines: Vec<_> = yaml_string_field(changelog, "run")?
        .lines()
        .map(str::trim)
        .collect();
    assert!(
        changelog_lines
            .iter()
            .any(|line| *line == "release_target=\"$(git rev-list -n 1 \"$release_tag\")\"")
    );
    assert!(changelog_lines.iter().any(|line| *line == "scripts/generate-release-changelog \"$release_tag\" \"$PREVIOUS_TAG\" > release-notes.md"));
    let release = yaml_step(publish_steps, "Create or update GitHub release")?;
    let release_lines: Vec<_> = yaml_string_field(release, "run")?
        .lines()
        .map(str::trim)
        .collect();
    assert!(release_lines.iter().any(|line| *line == "gh release edit \"$release_tag\" --title \"$release_tag\" --notes-file release-notes.md --target \"$RELEASE_TARGET\""));
    assert!(release_lines.iter().any(|line| *line == "gh release create \"$release_tag\" --title \"$release_tag\" --notes-file release-notes.md --target \"$RELEASE_TARGET\""));
    let upload = yaml_step(publish_steps, "Attach marketplace package to release")?;
    assert_eq!(
        yaml_string_field(upload, "run")?.trim(),
        "gh release upload \"$RELEASE_TAG\" \"dist/codexy-marketplace-plugin.tar.gz\" --clobber"
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

#[test]
fn touched_loc_workflow_runs_for_all_pull_requests() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/touched-loc-gate.yml"))?;
    let document: YamlValue = serde_yaml::from_str(&workflow)?;
    let workflow = document.as_mapping().ok_or("workflow root")?;
    let triggers = workflow
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == YamlValue::Bool(true))
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("workflow triggers")?;
    let pull_request = triggers.get("pull_request").ok_or("pull_request trigger")?;
    assert!(pull_request.is_null());
    let jobs = yaml_mapping_field(workflow, "jobs", "workflow")?;
    let job = yaml_mapping_field(jobs, "touched-loc", "jobs")?;
    let steps = yaml_steps(job)?;
    let checkout = yaml_step(steps, "Check out repository")?;
    let checkout_inputs = yaml_mapping_field(checkout, "with", "checkout step")?;
    assert_eq!(
        checkout_inputs
            .get("fetch-depth")
            .and_then(YamlValue::as_i64),
        Some(0)
    );
    let validation = yaml_step(steps, "Validate touched implementation LOC")?;
    assert_eq!(
        yaml_string_field(validation, "run")?,
        "scripts/validate-plugin-config --check-touched-loc --base-ref \"origin/${{ github.base_ref }}\""
    );
    Ok(())
}

#[test]
fn release_contract_uses_main_for_current_marketplace_ref() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let publish: JsonValue = serde_json::from_str(&std::fs::read_to_string(
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
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"])
    );
    Ok(())
}
