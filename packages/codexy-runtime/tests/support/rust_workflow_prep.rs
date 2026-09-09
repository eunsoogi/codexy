use std::fs;

use serde_yaml::Value;

use super::{WINDOWS_PREP_JOB, mapping_field, step_mappings, workflow_text};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CACHE_PATH: &str = "~/.cargo/registry\npackages/codexy-runtime/target\n";
const PREP_IF: &str = "github.event_name != 'workflow_dispatch' || inputs.run_mode == 'ci'";
const WINDOWS_IF: &str = "always() && (needs.windows-rust-prep.result == 'success' || (needs.windows-rust-prep.result == 'skipped' && github.event_name == 'workflow_dispatch' && inputs.run_mode == 'measurement'))";

#[test]
fn windows_preparation_shares_exact_cache_and_preserves_measurement_boundary() -> TestResult {
    let workflow = workflow_text()?;
    let document: Value = serde_yaml::from_str(&workflow)?;
    let jobs = mapping_field(document.as_mapping(), "jobs", "workflow")?;
    let prep = mapping_field(Some(jobs), WINDOWS_PREP_JOB, "jobs")?;
    assert_eq!(prep.get("if").and_then(Value::as_str), Some(PREP_IF));
    assert_eq!(prep.get("runs-on").and_then(Value::as_str), Some("windows-latest"));
    assert_eq!(prep.get("timeout-minutes").and_then(Value::as_u64), Some(5));
    assert!(!prep.contains_key("strategy"), "preparation must run once per Windows OS");

    assert!(step_mappings(prep).any(|step| {
        step.get("run")
            .and_then(Value::as_str)
            .is_some_and(|run| run.contains("build-windows-rust-test-artifact.ps1"))
    }));

    let cache = step_mappings(prep)
        .find(|step| step.get("uses").and_then(Value::as_str) == Some("actions/cache/restore@v5"))
        .ok_or("preparation cache restore")?;
    assert_eq!(cache.get("uses").and_then(Value::as_str), Some("actions/cache/restore@v5"));
    let with = cache.get("with").and_then(Value::as_mapping).ok_or("preparation cache inputs")?;
    assert_eq!(with.get("path").and_then(Value::as_str), Some(CACHE_PATH));
    let key = with.get("key").and_then(Value::as_str).ok_or("preparation cache key")?;
    assert!(key.contains("codexy-rust-rustup-toolchain-v3-"));
    assert!(key.contains("library and binaries"));
    assert!(key.contains("steps.rust_toolchain_identity.outputs.identity"));

    let upload = step_mappings(prep)
        .find(|step| step.get("uses").and_then(Value::as_str) == Some("actions/upload-artifact@v7"))
        .ok_or("preparation artifact upload")?;
    let upload_with = upload.get("with").and_then(Value::as_mapping).ok_or("artifact inputs")?;
    assert_eq!(
        upload_with.get("path").and_then(Value::as_str),
        Some("codexy-windows-rust-prepared.tar")
    );

    let windows = mapping_field(Some(jobs), "windows-rust-test", "jobs")?;
    assert_eq!(windows.get("needs").and_then(Value::as_str), Some(WINDOWS_PREP_JOB));
    assert_eq!(windows.get("if").and_then(Value::as_str), Some(WINDOWS_IF));
    assert!(workflow.contains("needs.windows-rust-prep.outputs.identity"));
    let download = step_mappings(windows)
        .find(|step| step.get("uses").and_then(Value::as_str) == Some("actions/download-artifact@v8"))
        .ok_or("preparation artifact download")?;
    assert!(download.get("if").and_then(Value::as_str).is_some_and(|value| {
        value.contains("inputs.run_mode == 'ci'") && value.contains("steps.rust-cache.outputs.cache-hit != 'true'")
    }));
    let restore = step_mappings(windows)
        .find(|step| step.get("run").and_then(Value::as_str).is_some_and(|run| run.contains("restore-windows-rust-test-artifact.ps1")))
        .ok_or("preparation artifact restore")?;
    assert!(restore.get("if").and_then(Value::as_str).is_some_and(|value| {
        value.contains("inputs.run_mode == 'ci'") && value.contains("steps.rust-cache.outputs.cache-hit != 'true'")
    }));

    let root = codexy_runtime::paths::repository_root();
    let archive = fs::read_to_string(root.join(".github/scripts/build-windows-rust-test-artifact.ps1"))?;
    assert!(archive.contains("cargo test --manifest-path $manifest --locked --no-run --lib --bins"));
    assert!(archive.contains("tar.exe -cf"));
    assert!(archive.contains(".cargo/registry"));
    assert!(archive.contains("packages/codexy-runtime/target"));
    let restore = fs::read_to_string(root.join(".github/scripts/restore-windows-rust-test-artifact.ps1"))?;
    assert!(restore.contains("tar.exe -xf $archive -C $HOME \".cargo/registry\""));
    assert!(restore.contains("tar.exe -xf $archive -C (Get-Location).Path \"packages/codexy-runtime/target\""));
    assert!(!restore.contains("strip-components"));
    Ok(())
}
