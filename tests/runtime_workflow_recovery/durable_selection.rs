use std::{fs, path::Path};

#[cfg(windows)]
use std::{path::PathBuf, process::Command};

#[cfg(windows)]
use sha2::{Digest as _, Sha256};

use serde_yaml::Value;

use crate::support;

#[test]
fn selected_runtime_verification_uses_the_immutable_release_after_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
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
        &[
            "gh release view \"$RELEASE_TAG\"",
            "runtime-release-receipt.json",
            "public release receipt does not match activated staging identity",
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
            "New-Item -ItemType Directory -Path dist -ErrorAction Stop",
            "if ($publicArchive)",
            "Copy-Item -LiteralPath $archive -Destination dist/codexy-marketplace-plugin.tar.gz",
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
    fs::create_dir_all(root.join("bin"))?;
    let archive = root.join("public.tar.gz");
    let receipt = root.join("public-receipt.json");
    fs::write(&archive, b"public release archive\n")?;
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
    let output = Command::new("pwsh").args(["-NoProfile", "-File"]).arg(&runner).current_dir(root).env("PUBLIC_ARCHIVE", archive).env("PUBLIC_RECEIPT", receipt).env("PATH", std::env::join_paths(paths)?).output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(root.join("public-release/codexy-marketplace-plugin.tar.gz").is_file());
    Ok(())
}

#[cfg(windows)]
fn windows_release_branch() -> Result<String, Box<dyn std::error::Error>> {
    let workflow: serde_yaml::Value = serde_yaml::from_str(&fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/plugin-runtime-binaries.yml"))?)?;
    let run = workflow["jobs"]["verify-windows-selected-candidate"]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Verify immutable native Windows candidate bytes")).and_then(|step| step["run"].as_str()).ok_or("windows verifier")?;
    let start = run.find("gh release view $env:RELEASE_TAG").ok_or("release view")?;
    let end = run.find("          $root =").ok_or("extraction")?;
    Ok(run[start..end].to_owned())
}
