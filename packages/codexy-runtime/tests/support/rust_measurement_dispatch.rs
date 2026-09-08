use std::{io, path::Path, process::{Command, Output}};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const IDENTITY_64: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const IDENTITY_65: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn measurement_validation_accepts_native_modes_and_rejects_invalid_inputs() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let head = current_head(root)?;
    let cases = [
        ("normal unprofiled", head.clone(), "normal", "false", true),
        ("normal profiled", head.clone(), "normal", "true", true),
        ("isolated", head.clone(), "isolated", "false", true),
        ("branch-like head", "main".to_owned(), "normal", "false", false),
        ("mismatched exact head", "0000000000000000000000000000000000000000".to_owned(), "normal", "false", false),
        ("malformed repeat", head.clone(), "normal", "false", false),
        ("malformed cache identity", head.clone(), "normal", "false", false),
        ("trailing LF repeat", head.clone(), "normal", "false", false),
        ("trailing LF cache identity", head.clone(), "normal", "false", false),
        ("repeat identity 64 accepted", head.clone(), "normal", "false", true),
        ("repeat identity 65 rejected", head.clone(), "normal", "false", false),
        ("cache identity 64 accepted", head.clone(), "normal", "false", true),
        ("cache identity 65 rejected", head.clone(), "normal", "false", false),
    ];
    for (name, candidate, mode, profiling, expected_success) in cases {
        let repeat = match name {
            "malformed repeat" => "normal cold",
            "trailing LF repeat" => "normal-cold-1\n",
            "repeat identity 64 accepted" => IDENTITY_64,
            "repeat identity 65 rejected" => IDENTITY_65,
            _ => "normal-cold-1",
        };
        let identity = match name {
            "malformed cache identity" => "normal pair",
            "trailing LF cache identity" => "normal-pair\n",
            "cache identity 64 accepted" => IDENTITY_64,
            "cache identity 65 rejected" => IDENTITY_65,
            _ => "normal-pair",
        };
        let temp = tempfile::tempdir()?;
        let env_file = temp.path().join("github_env");
        let output = run_helper(root, temp.path(), &env_file, &candidate, mode, profiling, repeat, identity)?;
        assert_eq!(output.status.success(), expected_success, "{name}: {}", String::from_utf8_lossy(&output.stderr));
        let measurement_root = temp.path().join("codexy-rust-measurement");
        if expected_success && (mode == "isolated" || profiling == "true") {
            let metadata = std::fs::read_to_string(measurement_root.join("metrics/measurement.txt"))?;
            for field in [
                format!("head_sha={candidate}"),
                "condition=cold".to_owned(),
                format!("cache_mode={mode}"),
                format!("repeat_id={repeat}"),
                format!("profiling={profiling}"),
                format!("cache_identity={identity}"),
            ] {
                assert!(metadata.lines().any(|line| line == field), "{name} lost {field}");
            }
            let emitted = std::fs::read_to_string(&env_file)?;
            if mode == "isolated" {
                for variable in ["CARGO_HOME=", "RUSTUP_HOME=", "CARGO_TARGET_DIR="] {
                    assert!(emitted.contains(variable), "{name} lost {variable}");
                }
            } else {
                assert!(emitted.contains("CODEXY_PROFILE_METRICS="));
                assert!(emitted.contains("CODEXY_PROFILE_COMMAND_METRICS_DIR="));
                assert!(!emitted.contains("CARGO_HOME="));
            }
        } else {
            assert!(!measurement_root.exists(), "{name} enabled instrumentation");
            assert!(!env_file.exists(), "{name} wrote instrumentation environment");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn run_helper(
    root: &Path,
    temp: &Path,
    env_file: &Path,
    head: &str,
    mode: &str,
    profiling: &str,
    repeat: &str,
    identity: &str,
) -> io::Result<Output> {
    Command::new("bash")
        .arg(root.join("scripts/prepare-rust-measurement.sh"))
        .current_dir(root)
        .env("HEAD_SHA", head)
        .env("CONDITION", "cold")
        .env("CACHE_MODE", mode)
        .env("REPEAT_ID", repeat)
        .env("CACHE_IDENTITY", identity)
        .env("PROFILE_ENABLED", profiling)
        .env("SHARD_INDEX", "0")
        .env("RUNNER_TEMP", temp)
        .env("GITHUB_ENV", env_file)
        .output()
}

#[cfg(windows)]
fn run_helper(
    root: &Path,
    temp: &Path,
    env_file: &Path,
    head: &str,
    mode: &str,
    profiling: &str,
    repeat: &str,
    identity: &str,
) -> io::Result<Output> {
    Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(root.join("scripts/prepare-rust-measurement.ps1"))
        .current_dir(root)
        .env("HEAD_SHA", head)
        .env("CONDITION", "cold")
        .env("CACHE_MODE", mode)
        .env("REPEAT_ID", repeat)
        .env("CACHE_IDENTITY", identity)
        .env("PROFILE_ENABLED", profiling)
        .env("SHARD_INDEX", "0")
        .env("RUNNER_TEMP", temp)
        .env("GITHUB_ENV", env_file)
        .output()
}

fn current_head(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
