use std::fs;

#[cfg(windows)]
use std::{path::PathBuf, process::Command};

#[cfg(windows)]
use sha2::{Digest as _, Sha256};

use serde_yaml::Value;

use crate::support;

#[test]
fn selected_runtime_verification_uses_the_immutable_release_after_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root()
        .join(".github/workflows/plugin-runtime-binaries.yml");
    let workflow: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let proof = workflow["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Download and verify selected immutable bytes")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("selected immutable runtime proof")?;
    support::assert_structured_literals(
        proof,
        "durable selected runtime verification",
        &["scripts/download-selected-runtime-package.sh dist/selected.tar.gz"],
    );
    let helper = fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts/download-selected-runtime-package.sh"),
    )?;
    support::assert_structured_literals(
        &helper,
        "durable selected runtime source helper",
        &[
            "gh release view \"$RELEASE_TAG\"",
            "grep -Eq 'HTTP 404|release not found' release-view-error",
            "runtime-release-receipt.json",
            "public release receipt does not match activated staging identity",
            "mkdir -p \"$marker_dir\"",
        ],
    );
    let assemble = workflow["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Assemble state-aware marketplace package without rebuilding")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("selected public-release projection")?;
    let public_start = assemble.find("public-release)").ok_or("public-release branch")?;
    let public_end = assemble[public_start..]
        .find("candidate-proven)")
        .map(|offset| public_start + offset)
        .ok_or("candidate-proven branch")?;
    let public = &assemble[public_start..public_end];
    support::assert_structured_literals(
        public,
        "durable public-release source projection",
        &[
            "public_receipt=public-release/runtime-release-receipt.json",
            "export STAGING_RUN_ID=\"$(jq -er .staging.runId \"$public_receipt\")\"",
            "export STAGING_SOURCE_COMMIT=\"$(jq -er .source.stagingSourceCommit \"$public_receipt\")\"",
            "export ACTIVATION_COMMIT=\"$(git rev-parse HEAD)\"",
            "export RELEASE_TAG=\"$(jq -er .release.tag \"$public_receipt\")\"",
            "export PUBLIC_RELEASE_RECEIPT=\"$public_receipt\"",
            "export PUBLIC_RELEASE=1",
            "scripts/materialize-runtime-release-archive dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz",
            "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz final-inspect/plugins/codexy-devtools public-release",
        ],
    );
    support::assert_structured_absent_literals(
        public,
        "durable public-release must not bypass current source projection",
        &[
            "cp dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz",
            ".candidate.artifact.stagingRunId .agents/plugins/runtime-activation.json",
            ".candidate.source.commit .agents/plugins/runtime-activation.json",
            ".runtime.selectedTag .agents/plugins/release-publish-contract.json",
            "export ACTIVATION_COMMIT=\"$(jq -er .source.activationCommit \"$public_receipt\")\"",
            "$env:ACTIVATION_COMMIT = \"$($receipt.source.activationCommit)\"",
        ],
    );
    let windows = workflow["jobs"]["verify-windows-selected-candidate"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Verify immutable native Windows candidate bytes")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("Windows selected immutable runtime proof")?;
    support::assert_structured_literals(
        windows,
        "Windows public-release archive projection",
        &[
            "$env:PUBLIC_RELEASE = \"1\"",
            "bash scripts/download-selected-runtime-package.sh $archive",
            "Test-Path -LiteralPath \"dist/public-release\" -PathType Leaf",
            "bash scripts/materialize-runtime-release-archive $archive dist/codexy-marketplace-plugin.tar.gz",
            "bash scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz \"$public/plugins/codexy-devtools\" public-release",
        ],
    );
    support::assert_structured_absent_literals(
        windows,
        "Windows selected helper owns dist creation",
        &["New-Item -ItemType Directory -Path dist -ErrorAction Stop"],
    );
    support::assert_structured_literals(
        windows,
        "Windows public-release receipt source projection",
        &[
            "$receiptPath = \"public-release/runtime-release-receipt.json\"",
            "$receipt = Get-Content -Raw $receiptPath | ConvertFrom-Json",
            "$env:STAGING_RUN_ID = \"$($receipt.staging.runId)\"",
            "$env:STAGING_SOURCE_COMMIT = \"$($receipt.source.stagingSourceCommit)\"",
            "$env:RELEASE_TAG = \"$($receipt.release.tag)\"",
            "$env:PUBLIC_RELEASE_RECEIPT = $receiptPath",
        ],
    );
    let materializer = fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts/materialize-runtime-release-archive"),
    )?;
    let source_helper = fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts/materialize_runtime_source.py"),
    )?;
    let materializer = format!("{materializer}\n{source_helper}");
    support::assert_structured_literals(
        &materializer,
        "public-release materializer receipt source",
        &[
            "PUBLIC_RELEASE_RECEIPT",
            "codexy-runtime-release-receipt/v1",
            "codexy-runtime-release-receipt/v2",
            "receipt_activation_commit",
            "SELECTED_CANDIDATE",
            "public runtime inventory",
        ],
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_selected_runtime_verification_exercises_the_public_release_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    fs::create_dir_all(root.join(".agents/plugins"))?;
    fs::create_dir_all(root.join("scripts"))?;
    fs::create_dir_all(root.join("bin"))?;
    fs::copy(
        codexy_runtime::paths::repository_root()
            .join("scripts/download-selected-runtime-package.sh"),
        root.join("scripts/download-selected-runtime-package.sh"),
    )?;
    let archive = root.join("public.tar.gz");
    let receipt = root.join("public-receipt.json");
    let archive_root = root.join("public archive/plugins/codexy-devtools/mcp");
    fs::create_dir_all(&archive_root)?;
    fs::write(archive_root.join("codexy-mcp-devtools.exe"), b"dispatcher\n")?;
    assert!(
        Command::new("tar")
            .current_dir(root.join("public archive"))
            .args(["-czf"])
            .arg(&archive)
            .arg("plugins/codexy-devtools")
            .status()?
            .success(),
        "fixture public archive creation failed"
    );
    fs::write(
        root.join(".agents/plugins/runtime-activation.json"),
        r#"{"candidate":{"source":{"commit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"artifact":{"stagingRunId":42,"stagingRunAttempt":3}},"provenance":{"runId":42}}"#,
    )?;
    fs::write(root.join(".agents/plugins/release-publish-contract.json"), r#"{"runtime":{"selectedTag":"v1.3.0"}}"#)?;
    let digest = format!("{:x}", Sha256::digest(fs::read(&archive)?));
    fs::write(&receipt, format!(r#"{{"release":{{"tag":"v1.3.0"}},"source":{{"stagingSourceCommit":"{}"}},"staging":{{"runId":42,"runAttempt":3}},"provenance":{{"runId":42}},"artifact":{{"sha256":"{digest}"}}}}"#, "a".repeat(40)))?;
    fs::write(root.join("bin/gh.cmd"), r#"@echo off
if "%1 %2"=="release view" exit /b 0
if "%1 %2"=="release download" (
  if not exist public-release mkdir public-release
  copy /y "%PUBLIC_ARCHIVE%" "public-release\codexy-marketplace-plugin.tar.gz" >nul
  copy /y "%PUBLIC_RECEIPT%" "public-release\runtime-release-receipt.json" >nul
  exit /b 0
)
exit /b 91
"#)?;
    let runner = root.join("verify-public-release.ps1");
    fs::write(&runner, format!("$record = Get-Content -Raw '.agents/plugins/runtime-activation.json' | ConvertFrom-Json\n$env:STAGING_RUN_ID = \"$($record.candidate.artifact.stagingRunId)\"\n$env:STAGING_SOURCE_COMMIT = \"$($record.candidate.source.commit)\"\n$env:RELEASE_TAG = 'v1.3.0'\n{}", windows_release_branch()?))?;
    let mut paths = vec![root.join("bin")];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?));
    let output = Command::new("pwsh").args(["-NoProfile", "-File"]).arg(&runner).current_dir(root).env("GITHUB_REPOSITORY", "eunsoogi/codexy").env("PUBLIC_ARCHIVE", archive).env("PUBLIC_RECEIPT", receipt).env("PATH", std::env::join_paths(paths)?).output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(root.join("public-release/codexy-marketplace-plugin.tar.gz").is_file());
    assert!(root.join("dist/public-release").is_file());
    Ok(())
}

#[cfg(windows)]
fn windows_release_branch() -> Result<String, Box<dyn std::error::Error>> {
    let workflow: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(PathBuf::from(codexy_runtime::paths::repository_root()).join(".github/workflows/plugin-runtime-binaries.yml"))?)?;
    let run = workflow["jobs"]["verify-windows-selected-candidate"]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Verify immutable native Windows candidate bytes")).and_then(|step| step["run"].as_str()).ok_or("windows verifier")?;
    let start = run.find("$archive = \"dist/selected.tar.gz\"").ok_or("archive")?;
    let end = run
        .find("$publicArchive = Test-Path -LiteralPath \"dist/public-release\"")
        .ok_or("public-release marker")?;
    Ok(run[start..end].to_owned())
}
