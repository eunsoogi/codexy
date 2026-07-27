use std::path::Path;
use std::process::Command;

#[test]
fn windows_gate_reconciles_exact_compiled_test_names() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("exact")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&vec!["3", "3", "PASS"]));
    assert_eq!(report.get("coverage-missing"), Some(&vec!["0"]));
    assert_eq!(report.get("coverage-duplicate-or-extra"), Some(&vec!["0"]));
    Ok(())
}

#[test]
fn windows_gate_accounts_for_unrecognized_compiled_targets() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_case("unknown-target")?;
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    let report = report_fields(&stdout);
    assert_eq!(report.get("coverage-tests"), Some(&vec!["4", "4", "PASS"]));
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
fn windows_gate_rejects_missing_duplicate_extra_and_failed_inventory() -> Result<(), Box<dyn std::error::Error>> {
    for case in ["missing", "duplicate", "extra", "list-fails", "run-fails"] {
        let output = run_case(case)?;
        assert!(!output.status.success(), "{case}: {output:?}");
    }
    Ok(())
}

fn run_case(case: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    let cargo = bin.join("cargo");
    std::fs::write(
        &cargo,
        "#!/bin/sh\ncase \"$1\" in\nmetadata) [ \"$PROFILE_CASE\" = metadata-fails ] && exit 9; printf '%s\\n' '{\"packages\":[{\"targets\":[{\"kind\":[\"test\"]}]}]}' ;;\ntest) case \"$*\" in\n*--list*) [ \"$PROFILE_CASE\" = list-fails ] && exit 9; printf '%s\\n' 'Running unittests src/lib.rs (target/debug/lib)' 'alpha: test' 'panic - should panic: test' 'Running tests/suites/all.rs (target/debug/all)' 'beta: test'; case \"$PROFILE_CASE\" in unknown-target) printf '%s\\n' 'Running tests/custom.rs (target/debug/custom)' 'custom: test' ;; esac ;;\n*) printf '%s\\n' 'Running unittests src/lib.rs (target/debug/lib)' 'test alpha ... ok' 'test panic - should panic ... ok' 'Running tests/suites/all.rs (target/debug/all)'; case \"$PROFILE_CASE\" in missing) : ;; *) printf '%s\\n' 'test beta ... ok' ;; esac; case \"$PROFILE_CASE\" in unknown-target) printf '%s\\n' 'Running tests/custom.rs (target/debug/custom)' 'test custom ... ok' ;; esac; case \"$PROFILE_CASE\" in duplicate) printf '%s\\n' 'test beta ... ok' ;; extra) printf '%s\\n' 'test extra ... ok' ;; run-fails) printf '%s\\n' 'test beta ... FAILED'; exit 101 ;; esac; printf '%s\\n' 'Finished `test` profile [unoptimized] target(s) in 0.01s' 'test result: ok. 1802 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s' ;; esac ;;\nesac\n",
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
    Ok(Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests"))
        .args(["--root", env!("CARGO_MANIFEST_DIR"), "--windows"])
        .env("PATH", path)
        .env("PROFILE_CASE", case)
        .output()?)
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
