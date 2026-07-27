use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "windows_measurement/accounting.rs"]
mod accounting;

#[test]
fn windows_measurement_rejects_missing_or_duplicate_test_coverage(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inventory = temp.path().join("inventory.json");
    let profiles = temp.path().join("profiles");
    std::fs::create_dir(&profiles)?;
    std::fs::write(
        &inventory,
        r#"{"tests":["suite_all::agent::one","suite_archive::two"]}"#,
    )?;
    write_profile(&profiles.join("agent.json"), &["suite_all::agent::one"])?;
    assert!(!verify(&inventory, &profiles, &temp.path().join("missing.json"))?
        .status
        .success());
    write_profile(&profiles.join("archive.json"), &["suite_archive::two"])?;

    let output = verify(&inventory, &profiles, &temp.path().join("coverage.json"))?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    write_profile(&profiles.join("duplicate.json"), &["suite_all::agent::one"])?;
    assert!(!verify(&inventory, &profiles, &temp.path().join("duplicate.json"))?
        .status
        .success());
    Ok(())
}

#[test]
fn windows_measurement_parses_targeted_cargo_inventory() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    write_fake_cargo(
        &bin,
        "#!/bin/sh\nprintf '%s\\n' '     Running unittests src/lib.rs (target/debug/deps/lib)' 'lib_case: test' '     Running tests/suites/all.rs (target/debug/deps/all)' 'agent::case: test'\n",
        "@echo off\r\necho      Running unittests src/lib.rs (target\\debug\\deps\\lib)\r\necho lib_case: test\r\necho      Running tests/suites/all.rs (target\\debug\\deps\\all)\r\necho agent::case: test\r\n",
    )?;
    let inventory = temp.path().join("inventory.json");
    let path = prepend_path(&bin)?;
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["inventory", "--artifact"])
        .arg(&inventory)
        .env("PATH", path)
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let inventory: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(inventory)?)?;
    assert_eq!(
        inventory["tests"],
        serde_json::json!(["lib::lib_case", "suite_all::agent::case"])
    );
    Ok(())
}

#[test]
fn windows_measurement_records_a_cluster_execution() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    write_fake_cargo(
        &bin,
        "#!/bin/sh\ncase \"$*\" in *--list*) printf '%s\\n' 'agent::case: test' ;; *) printf '%s\\n' '   Compiling fixture v0.0.0' '    Finished `test` profile [unoptimized] target(s) in 0.01s' 'test agent::case ... ok' 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s' ;; esac\n",
        "@echo off\r\necho    Compiling fixture v0.0.0\r\necho     Finished `test` profile [unoptimized] target(s) in 0.01s\r\necho test agent::case ... ok\r\necho test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\r\n",
    )?;
    let artifact = temp.path().join("agent.json");
    let inventory = temp.path().join("inventory.json");
    std::fs::write(&inventory, r#"{"tests":["suite_all::agent::case"]}"#)?;
    let path = prepend_path(&bin)?;
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["run", "--name", "suite-all-agent", "--artifact"])
        .arg(&artifact)
        .args(["--inventory"])
        .arg(&inventory)
        .env("PATH", path)
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let artifact: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(artifact)?)?;
    assert_eq!(artifact["tests"], serde_json::json!(["suite_all::agent::case"]));
    assert_eq!(artifact["passed"], serde_json::json!(1));
    Ok(())
}

#[test]
fn windows_measurement_persists_failed_cluster_streams_and_partial_timing(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    write_fake_cargo(
        &bin,
        "#!/bin/sh\ncase \"$*\" in *--list*) printf '%s\\n' 'agent::failure: test' ;; *) printf '%s\\n' 'test agent::failure ... FAILED' 'test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s'; printf '%s\\n' 'fixture stderr' >&2; exit 101 ;; esac\n",
        "@echo off\r\necho test agent::failure ... FAILED\r\necho test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\r\necho fixture stderr 1>&2\r\nexit /b 101\r\n",
    )?;
    let artifact = temp.path().join("failed.json");
    let inventory = temp.path().join("inventory.json");
    std::fs::write(&inventory, r#"{"tests":["suite_all::agent::failure"]}"#)?;
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["run", "--name", "suite-all-agent", "--artifact"])
        .arg(&artifact)
        .args(["--inventory"])
        .arg(&inventory)
        .env("PATH", prepend_path(&bin)?)
        .output()?;
    assert_eq!(output.status.code(), Some(101));
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(artifact)?)?;
    assert_eq!(value["exitCode"], 101);
    assert!(value["durationSeconds"].as_f64().is_some());
    assert_eq!(
        value["stdout"],
        serde_json::json!(
            "test agent::failure ... FAILED\n\
             test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"
        )
    );
    assert_eq!(value["stderr"], serde_json::json!("fixture stderr\n"));
    Ok(())
}

#[test]
fn windows_measurement_workflow_keeps_the_required_gate_at_ten_minutes() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust-test.yml"),
    )?;
    crate::support::assert_structured_literals(
        &workflow,
        "windows-rust-measurement-workflow",
        &[
            "windows-rust-test:",
            "timeout-minutes: 10",
            "windows-rust-profile-inventory:",
            "windows-rust-profile:",
            "windows-rust-profile-coverage:",
            "actions/upload-artifact@v7",
        ],
    );
    assert_eq!(workflow.matches("cargo test --locked --all-targets").count(), 1);
    assert_eq!(
        workflow
            .matches("scripts/install-windows-test-prerequisites.ps1")
            .count(),
        2,
        "the required and profiled Windows workloads must share one prerequisite installer"
    );
    Ok(())
}

fn prepend_path(bin: &Path) -> Result<std::ffi::OsString, Box<dyn std::error::Error>> {
    let current = std::env::var_os("PATH").ok_or("PATH")?;
    let paths = std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&current));
    Ok(std::env::join_paths(paths)?)
}

#[cfg(unix)]
fn write_fake_cargo(
    bin: &Path,
    unix_contents: &str,
    _windows_contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cargo = bin.join("cargo");
    std::fs::write(&cargo, unix_contents)?;
    crate::support::make_executable(&cargo)?;
    Ok(cargo)
}

#[cfg(windows)]
fn write_fake_cargo(
    bin: &Path,
    _unix_contents: &str,
    windows_contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cargo = bin.join("cargo.cmd");
    std::fs::write(&cargo, windows_contents)?;
    Ok(cargo)
}

fn write_profile(path: &Path, tests: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let target = tests
        .first()
        .ok_or("profile test")?
        .split("::")
        .next()
        .ok_or("profile target")?;
    let tests = serde_json::to_string(tests)?;
    std::fs::write(
        path,
        format!(
            "{{\"name\":\"fixture\",\"target\":\"{target}\",\"exitCode\":0,\"passed\":1,\"failed\":0,\"ignored\":0,\"tests\":{tests},\"durationSeconds\":1.0,\"metrics\":{{}}}}"
        ),
    )?;
    Ok(())
}

fn verify(
    inventory: &Path,
    profiles: &Path,
    coverage: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-windows-rust"))
        .args(["verify", "--inventory"])
        .arg(inventory)
        .args(["--profiles"])
        .arg(profiles)
        .args(["--coverage"])
        .arg(coverage)
        .output()?)
}
