use std::{fs, path::Path};

use serde_json::{json, Value};

use super::isolation::{
    bootstrap_candidate_version, fixture_version, next_patch_version, version_surface_contents,
};

#[test]
fn historical_readmes_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, selected) = super::selected_fixture(&temp, "historical-readme")?;
    let prior = super::previous_patch_version(&selected)?;
    for (path, bytes) in version_surface_contents(&root)? {
        fs::write(path, String::from_utf8(bytes)?.replace(&selected, &prior))?;
    }
    let mut expected = Vec::new();
    for relative in ["README.md", "README.ko.md"] {
        let path = root.join(relative);
        let mutated = fs::read_to_string(&path)?.replace(
            &format!("{} --ref v{selected}", super::README_COMMAND),
            super::README_COMMAND,
        );
        fs::write(&path, mutated.as_bytes())?;
        expected.push((path, mutated.into_bytes()));
    }
    assert!(!super::run_sync_script(&root, &["--check"])?
        .status
        .success());
    for (path, bytes) in expected {
        assert_eq!(fs::read(path)?, bytes);
    }
    Ok(())
}

#[test]
fn version_admission_matrix_is_ordered_and_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = super::shared_repository_archive()?;
    let current = super::archive_repository(archive, &temp, "current")?;
    let current_version = fixture_version(&current)?;
    let non_selected_version = next_patch_version(&current_version)?;
    for (version, accepted) in [(&current_version, true), (&non_selected_version, false)] {
        assert_eq!(
            super::run_sync(&current, &["--admit-version", version])?
                .status
                .success(),
            accepted
        );
    }

    for case in [
        "exact",
        "stale-bootstrap",
        "stale-runtime",
        "legacy-runtime",
        "wrapper-drift",
    ] {
        let root = super::archive_repository(archive, &temp, case)?;
        let target = next_patch_version(&current_version)?;
        select_next_public_identities(&root, &target, &current_version)?;
        match case {
            "exact" => {}
            "stale-bootstrap" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["bootstrap"]["selectedVersion"] = json!(current_version.clone()),
            )?,
            "stale-runtime" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["runtime"]["selectedTag"] = json!(format!("v{current_version}")),
            )?,
            "legacy-runtime" => mutate_json(
                &root.join(".agents/plugins/runtime-activation.json"),
                |value| value["candidate"]["artifact"]["stagingRunId"] = json!(false),
            )?,
            "wrapper-drift" => fs::write(
                root.join("plugins/codexy-devtools/mcp/codexy-mcp-devtools"),
                format!("#!/bin/sh\nexec uvx --from getcodexy=={target} codexy-mcp-runtime \"$server\" -- \"$@\"\n"),
            )?,
            other => return Err(format!("unknown admission case: {other}").into()),
        }
        let output = super::run_sync(&root, &["--admit-version", &target])?;
        assert_eq!(
            output.status.success(),
            case == "exact",
            "unexpected admission result for {case}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        if case == "exact" {
            let advance = super::run_sync(&root, &["--version", &target])?;
            assert!(
                advance.status.success(),
                "activated source failed sync --version: {}",
                String::from_utf8_lossy(&advance.stderr),
            );
            let check = super::run_sync(&root, &["--check"])?;
            assert!(
                check.status.success(),
                "activated source with its preserved runtime selection failed sync --check: {}",
                String::from_utf8_lossy(&check.stderr),
            );
        }
    }
    Ok(())
}

#[test]
fn candidate_preparation_keeps_selected_identity_until_activation(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = super::shared_repository_archive()?;
    let root = super::archive_repository(archive, &temp, "candidate")?;
    let selected_version = fixture_version(&root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    let selected = super::run_sync(&root, &["--version", &selected_version])?;
    assert!(
        selected.status.success(),
        "selected fixture normalization failed: {}",
        String::from_utf8_lossy(&selected.stderr)
    );
    let bootstrap = root.join("packages/codexy-runtime/src/version/bootstrap.rs");
    let bootstrap_text = fs::read_to_string(&bootstrap)?;
    let current_candidate = bootstrap_candidate_version(&root)?;
    fs::write(
        &bootstrap,
        bootstrap_text.replace(
            &format!("CANDIDATE_VERSION: &str = \"{current_candidate}\""),
            &format!("CANDIDATE_VERSION: &str = \"{selected_version}\""),
        ),
    )?;
    let contract_path = root.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path)?)?;
    contract["bootstrap"]["candidateVersion"] = json!(selected_version);
    fs::write(
        &contract_path,
        format!("{}\n", serde_json::to_string_pretty(&contract)?),
    )?;
    let before = version_surface_contents(&root)?;
    let bootstrap_before = fs::read(&bootstrap)?;
    let pyproject = root.join("packages/getcodexy/pyproject.toml");
    let uv_lock = root.join("packages/getcodexy/uv.lock");
    let pyproject_before = fs::read(&pyproject)?;
    let uv_lock_before = fs::read(&uv_lock)?;
    let contract_before = fs::read(&contract_path)?;
    for args in [
        ["--admit-candidate", candidate_version.as_str()],
        ["--prepare-candidate", candidate_version.as_str()],
    ] {
        let output = super::run_sync(&root, &args)?;
        assert!(
            output.status.success(),
            "candidate command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if args[0] == "--admit-candidate" {
            assert_eq!(version_surface_contents(&root)?, before);
            assert_eq!(fs::read(&bootstrap)?, bootstrap_before);
        }
    }
    let runtime_release: Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    let selected_runtime_tag = runtime_release["artifact"]["tag"]
        .as_str()
        .ok_or("selected runtime tag")?;
    let contract: Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
    assert_eq!(contract["runtime"]["selectedTag"], selected_runtime_tag);
    let check = super::run_sync(&root, &["--check-candidate"])?;
    assert!(
        check.status.success(),
        "candidate check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["version"], selected_version);
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    assert_eq!(contract["bootstrap"]["candidateVersion"], candidate_version);
    assert_eq!(contract["runtime"]["selectedTag"], selected_runtime_tag);
    assert!(fs::read_to_string(&bootstrap)?
        .contains(&format!("VERSION: &str = \"{selected_version}\"")));
    assert!(fs::read_to_string(&bootstrap)?.contains(&format!(
        "CANDIDATE_VERSION: &str = \"{candidate_version}\""
    )));
    assert_ne!(fs::read(&bootstrap)?, bootstrap_before);
    assert_ne!(fs::read(&pyproject)?, pyproject_before);
    assert_ne!(fs::read(&uv_lock)?, uv_lock_before);
    assert_ne!(fs::read(&contract_path)?, contract_before);
    for relative in ["README.md", "README.ko.md"] {
        let readme = fs::read_to_string(root.join(relative))?;
        assert!(
            readme.contains(&format!("--ref v{candidate_version}")),
            "candidate pin is stale in {relative}"
        );
    }
    for (path, bytes) in before {
        if path.ends_with("release-publish-contract.json")
            || path.ends_with("packages/getcodexy/pyproject.toml")
            || path.ends_with("packages/getcodexy/uv.lock")
        {
            continue;
        }
        if path.ends_with("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json") {
            let manifest: Value = serde_json::from_slice(&fs::read(&path)?)?;
            for field in ["components", "compatibleCombinations"] {
                for entry in manifest[field]
                    .as_array()
                    .ok_or("component manifest array")?
                {
                    assert_eq!(entry["version"], candidate_version);
                }
            }
            continue;
        }
        assert_eq!(fs::read(path)?, bytes);
    }
    Ok(())
}

fn select_next_public_identities(
    root: &Path,
    target: &str,
    candidate: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    mutate_json(
        &root.join(".agents/plugins/release-publish-contract.json"),
        |value| {
            value["bootstrap"]["selectedVersion"] = json!(target);
            value["runtime"]["selectedTag"] = json!(format!("v{target}"));
        },
    )?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        format!(
            "pub(super) const VERSION: &str = \"{target}\";\npub(super) const CANDIDATE_VERSION: &str = \"{candidate}\";\n"
        ),
    )?;
    Ok(())
}

fn mutate_json(
    path: &Path,
    mutation: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    mutation(&mut value);
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&value)?))?;
    Ok(())
}
