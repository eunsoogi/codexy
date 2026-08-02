use std::path::Path;
use std::process::Command;

#[test]
fn windows_gate_reconciles_exact_compiled_test_names() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("exact")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&vec!["1803", "1803", "PASS"]));
    assert_eq!(report.get("coverage-missing"), Some(&vec!["0"]));
    assert_eq!(report.get("coverage-duplicate-or-extra"), Some(&vec!["0"]));
    assert_eq!(report.get("budget-seconds"), Some(&vec!["180.000", "PASS"]));
    Ok(())
}

#[test]
fn windows_gate_counts_a_crlf_fragmented_completed_status() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("fragmented")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&vec!["1803", "1803", "PASS"]));
    assert_eq!(report.get("coverage-missing"), Some(&vec!["0"]));
    Ok(())
}

#[test]
fn windows_gate_rejects_a_fragmented_status_at_true_eof() -> Result<(), Box<dyn std::error::Error>> {
    assert_rejected_fragmented_case("fragmented-eof", ["1803", "2", "FAIL"], "1801", "0", "0")
}

#[test]
fn windows_gate_resets_a_fragmented_status_at_a_new_target() -> Result<(), Box<dyn std::error::Error>> {
    assert_rejected_fragmented_case("fragmented-new-target", ["1804", "2", "FAIL"], "1802", "0", "0")
}

#[test]
fn windows_gate_resets_a_fragmented_status_at_a_complete_status() -> Result<(), Box<dyn std::error::Error>> {
    assert_rejected_fragmented_case("fragmented-new-status", ["1803", "3", "FAIL"], "1800", "0", "0")
}

#[test]
fn windows_gate_rejects_a_foreign_line_without_a_test_prefix() -> Result<(), Box<dyn std::error::Error>> {
    assert_rejected_fragmented_case("foreign-line", ["1803", "2", "FAIL"], "1801", "0", "0")
}

#[test]
fn windows_gate_accounts_for_unrecognized_compiled_targets() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("unknown-target")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&vec!["1804", "1804", "PASS"]));
    Ok(())
}

#[test]
fn windows_gate_does_not_depend_on_a_separate_cargo_metadata_probe(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("metadata-fails")?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_gate_uses_one_cargo_workload_and_lists_completed_test_binaries(
) -> Result<(), Box<dyn std::error::Error>> {
    let (output, commands) = run_case_with_commands("exact")?;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(commands.lines().collect::<Vec<_>>(), ["test --locked --all-targets"]);
    Ok(())
}

#[test]
fn windows_gate_emits_structured_execution_and_fixture_telemetry(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("telemetry")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    let telemetry: serde_json::Value = serde_json::from_str(
        report
            .get("windows-telemetry-json")
            .and_then(|fields| fields.first())
            .ok_or("windows telemetry")?,
    )?;
    for field in [
        "temp",
        "tmp",
        "runner_temp",
        "workspace",
        "target",
        "logical_cpus",
        "available_parallelism",
        "rust_test_threads",
        "fixture_materializations",
        "fixture_copied_files",
        "fixture_copied_bytes",
    ] {
        assert!(telemetry.get(field).is_some(), "missing {field}: {telemetry}");
    }
    assert_eq!(telemetry["fixture_materializations"].as_u64(), Some(1));
    assert_eq!(telemetry["fixture_copied_files"].as_u64(), Some(2));
    assert_eq!(telemetry["fixture_copied_bytes"].as_u64(), Some(17));
    Ok(())
}

#[test]
fn windows_gate_counts_observed_test_outcomes_without_target_summaries(
) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("no-summary")?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[test]
fn windows_gate_rejects_missing_duplicate_extra_and_failed_inventory() -> Result<(), Box<dyn std::error::Error>> {
    for case in ["missing", "duplicate", "extra", "list-fails", "run-fails"] {
        let output = run_case(case)?;
        assert!(!output.status.success(), "{case}: {output:?}");
    }
    Ok(())
}

#[test]
fn windows_gate_rejects_a_declared_compiled_target_that_never_starts()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("unstarted-target")?;
    assert!(
        !output.status.success(),
        "coverage must reject a compiled target omitted after the workload starts: {output:?}"
    );
    Ok(())
}

fn run_case(case: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(run_case_with_commands(case)?.0)
}

fn run_case_with_commands(
    case: &str,
) -> Result<(std::process::Output, String), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = fixture_root(temp.path())?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    let commands = temp.path().join("cargo-commands");
    let lib_test = bin.join("lib-test");
    let all_test = bin.join("all-test");
    let custom_test = bin.join("custom-test");
    let archive_test = root.join("target/debug/deps/suite_archive-fixture");
    write_executable(
        &lib_test,
        "#!/bin/sh\n[ \"$PROFILE_CASE\" = list-fails ] && exit 9\nprintf '%s\\n' 'alpha: test' 'panic - should panic: test'\n",
    )?;
    write_executable(
        &all_test,
        "#!/bin/sh\nprintf '%s\\n' 'beta: test'\nindex=1\nwhile [ \"$index\" -le 1799 ]; do\n  printf 'generated-%s: test\\n' \"$index\"\n  index=$((index + 1))\ndone\n",
    )?;
    write_executable(&custom_test, "#!/bin/sh\nprintf '%s\\n' 'custom: test'\n")?;
    write_executable(&archive_test, "#!/bin/sh\nprintf '%s\\n' 'archive: test'\n")?;
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$PROFILE_MARKER\"\ncase \"$1\" in\nmetadata) [ \"$PROFILE_CASE\" = metadata-fails ] && exit 9; printf '%s\\n' '{\"packages\":[{\"targets\":[{\"kind\":[\"test\"]}]}]}' ;;\ntest) [ \"$PROFILE_CASE\" = telemetry ] && printf 'fixture-materialization\\t2\\t17\\n' >> \"$CODEXY_WINDOWS_PROFILE_METRICS\"; printf '%s\\n' \"     Running unittests src/lib.rs ($PROFILE_LIB_TEST)\" 'test alpha ... ok' 'test panic - should panic ... ok' \"     Running tests/suites/all.rs ($PROFILE_ALL_TEST)\"; case \"$PROFILE_CASE\" in missing) : ;; fragmented) printf 'test beta ... foreign complete line\\r\\nok\\r\\n' ;; fragmented-eof) printf 'test beta ... foreign complete line\\r\\n'; exit 0 ;; fragmented-new-target) printf 'test beta ... foreign complete line\\r\\n'; printf '%s\\r\\n' \"     Running tests/custom.rs ($PROFILE_CUSTOM_TEST)\" 'ok'; exit 0 ;; fragmented-new-status) printf 'test beta ... foreign complete line\\r\\ntest generated-1 ... ok\\r\\nok\\r\\n'; exit 0 ;; foreign-line) printf 'foreign complete line\\r\\nok\\r\\n'; exit 0 ;; *) printf '%s\\n' 'test beta ... ok' ;; esac; index=1; while [ \"$index\" -le 1799 ]; do printf 'test generated-%s ... ok\\n' \"$index\"; index=$((index + 1)); done; case \"$PROFILE_CASE\" in unknown-target) printf '%s\\n' \"     Running tests/custom.rs ($PROFILE_CUSTOM_TEST)\" 'test custom ... ok' ;; duplicate) printf '%s\\n' 'test beta ... ok' ;; extra) printf '%s\\n' 'test extra ... ok' ;; run-fails) printf '%s\\n' 'test beta ... FAILED'; exit 101 ;; esac; case \"$PROFILE_CASE\" in unstarted-target) : ;; *) printf '%s\\n' \"     Running tests/suites/archive.rs ($PROFILE_ARCHIVE_TEST)\" 'test archive ... ok' ;; esac; case \"$PROFILE_CASE\" in no-summary) : ;; *) printf '%s\\n' 'Finished `test` profile [unoptimized] target(s) in 0.01s' 'test result: ok. 1803 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s' ;; esac ;;\nesac\n",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&cargo)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo, permissions)?;
    }
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?)),
    )?;
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests"))
        .args(["--root", root.to_str().ok_or("fixture root")?, "--windows"])
        .env("PATH", path)
        .env("PROFILE_CASE", case)
        .env("RUNNER_TEMP", temp.path())
        .env("PROFILE_MARKER", &commands)
        .env("PROFILE_LIB_TEST", &lib_test)
        .env("PROFILE_ALL_TEST", &all_test)
        .env("PROFILE_CUSTOM_TEST", &custom_test)
        .env("PROFILE_ARCHIVE_TEST", &archive_test)
        .output()?;
    Ok((output, std::fs::read_to_string(commands)?))
}

fn fixture_root(temp: &Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = temp.join("root");
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::create_dir_all(root.join("tests/support"))?;
    std::fs::create_dir_all(root.join("target/debug/deps"))?;
    std::fs::create_dir_all(root.join(".github/workflows"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"profile-fixture\"\nversion = \"0.0.0\"\n\n[[test]]\nname = \"suite_all\"\npath = \"tests/suites/all.rs\"\n\n[[test]]\nname = \"suite_archive\"\npath = \"tests/suites/archive.rs\"\n",
    )?;
    std::fs::write(root.join("tests/support/release_archive.rs"), "")?;
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/rust-test.yml"),
        root.join(".github/workflows/rust-test.yml"),
    )?;
    Ok(root)
}

fn write_executable(path: &Path, contents: &str) -> std::io::Result<()> {
    std::fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn report_fields(output: &str) -> std::collections::BTreeMap<&str, Vec<&str>> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            fields.next().map(|name| (name, fields.collect()))
        })
        .collect()
}

fn assert_rejected_fragmented_case(
    case: &str,
    coverage_tests: [&str; 3],
    missing: &str,
    duplicates: &str,
    extra: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case(case)?;
    assert!(!output.status.success(), "{case}: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&coverage_tests.to_vec()), "{case}");
    assert_eq!(report.get("coverage-missing"), Some(&vec![missing]), "{case}");
    assert_eq!(report.get("coverage-duplicates"), Some(&vec![duplicates]), "{case}");
    assert_eq!(report.get("coverage-extra"), Some(&vec![extra]), "{case}");
    Ok(())
}
