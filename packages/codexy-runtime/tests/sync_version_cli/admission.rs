use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};

#[test]
fn version_admission_matrix_is_ordered_and_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = super::shared_repository_archive()?;
    let current = super::archive_repository(archive, &temp, "current")?;
    assert!(admit(&current, "1.3.0")?.status.success());
    assert!(!admit(&current, "1.1.0")?.status.success());

    for case in ["exact", "stale-bootstrap", "stale-runtime", "legacy-runtime", "wrapper-drift"] {
        let root = super::archive_repository(archive, &temp, case)?;
        select_next_public_identities(&root)?;
        match case {
            "exact" => {}
            "stale-bootstrap" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["bootstrap"]["selectedVersion"] = json!("1.3.0"),
            )?,
            "stale-runtime" => mutate_json(
                &root.join(".agents/plugins/release-publish-contract.json"),
                |value| value["runtime"]["selectedTag"] = json!("v1.3.0"),
            )?,
            "legacy-runtime" => mutate_json(
                &root.join(".agents/plugins/runtime-activation.json"),
                |value| value["candidate"]["artifact"]["stagingRunId"] = json!(false),
            )?,
            "wrapper-drift" => fs::write(
                root.join("plugins/codexy-devtools/mcp/codexy-mcp-devtools"),
                "#!/bin/sh\nexec uvx --from getcodexy==1.2.1 codexy-mcp-runtime \"$server\" -- \"$@\"\n",
            )?,
            other => return Err(format!("unknown admission case: {other}").into()),
        }
        let output = admit(&root, "1.3.1")?;
        assert_eq!(
            output.status.success(),
            case == "exact",
            "unexpected admission result for {case}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        if case == "exact" {
            let advance = sync_version(&root, "1.3.1")?;
            assert!(
                advance.status.success(),
                "activated source failed sync --version: {}",
                String::from_utf8_lossy(&advance.stderr),
            );
            let check = sync_check(&root)?;
            assert!(
                check.status.success(),
                "activated source with its preserved runtime selection failed sync --check: {}",
                String::from_utf8_lossy(&check.stderr),
            );
        }
    }
    Ok(())
}

fn select_next_public_identities(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    mutate_json(
        &root.join(".agents/plugins/release-publish-contract.json"),
        |value| {
            value["bootstrap"]["selectedVersion"] = json!("1.3.1");
            value["runtime"]["selectedTag"] = json!("v1.3.1");
        },
    )?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        "pub(super) const VERSION: &str = \"1.3.1\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n",
    )?;
    Ok(())
}

fn admit(root: &Path, version: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--admit-version", version])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}

fn sync_check(root: &Path) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}

fn sync_version(root: &Path, version: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", version])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
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
