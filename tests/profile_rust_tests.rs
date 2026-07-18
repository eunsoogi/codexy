use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
#[test]
fn gate_runs_the_exact_full_workload_once() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let output = fixture.run(&[])?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("integration-targets\t2\tPASS"), "{stdout}");
    assert!(
        stdout.contains("tests\t1802 passed\t0 failed\t0 ignored\tPASS"),
        "{stdout}"
    );
    assert!(stdout.contains("nested-builds\t0\tPASS"), "{stdout}");
    assert!(stdout.contains("compile-seconds\t62.000"), "{stdout}");
    assert!(stdout.contains("budget-seconds\t180.000"), "{stdout}");
    assert!(stdout.ends_with("result\tPASS\n"), "{stdout}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_propagates_a_single_full_workload_failure() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(42, 1802, 0)?;
    let output = fixture.run(&[])?;

    assert_eq!(output.status.code(), Some(42), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_fails_an_exact_workload_over_180_seconds_without_sleeping()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let clock = fixture.temp.path().join("clock");
    std::fs::create_dir(&clock)?;
    std::fs::write(
        clock.join("sitecustomize.py"),
        "import time\n_values = iter((0.0, 181.0))\ntime.perf_counter = lambda: next(_values)\n",
    )?;
    let output = fixture.run(&[("PYTHONPATH", clock.as_os_str())])?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    assert!(String::from_utf8(output.stdout)?.contains("result\tFAIL"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_coverage_loss_and_ignored_tests() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1801, 1)?;
    let output = fixture.run(&[])?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("tests\t1801 passed\t0 failed\t1 ignored\tFAIL"),
        "{stdout}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_has_no_relaxable_budget_option() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let output = fixture.run(&[("EXTRA_ARGUMENT", std::ffi::OsStr::new("--max-seconds"))])?;

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8(output.stderr)?.contains("unrecognized arguments"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_timeout_or_profiler_in_an_unrelated_job() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  unrelated:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  rust-test:\n    timeout-minutes: 10\n    steps:\n      - run: echo not-the-gate\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_second_profiler_invocation_in_another_job(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n  unrelated:\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_contract_fields_leaked_from_an_underscore_job(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    steps:\n      - run: echo not-the-gate\n  _unrelated:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_rejects_a_block_scalar_that_only_mentions_the_profiler(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    env: |\n      run: scripts/profile-rust-tests\n    steps:\n      - run: echo not-the-gate\n",
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
#[test]
fn gate_accepts_the_profiler_step_after_a_blank_line() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - name: setup\n        run: echo setup\n\n      - run: scripts/profile-rust-tests\n",
    )?;
    assert!(fixture.run(&[])?.status.success());
    Ok(())
}

#[cfg(unix)]
struct GateFixture {
    temp: tempfile::TempDir,
    marker: PathBuf,
    bin_dir: PathBuf,
    workflow: PathBuf,
}

#[cfg(unix)]
impl GateFixture {
    fn new(exit: i32, passed: usize, ignored: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bin_dir = temp.path().join("bin");
        std::fs::create_dir(&bin_dir)?;
        let marker = temp.path().join("workloads");
        let cargo = bin_dir.join("cargo");
        write_executable(
            &cargo,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = metadata ]; then\n  printf '%s\\n' '{{\"packages\":[{{\"targets\":[{{\"kind\":[\"test\"]}},{{\"kind\":[\"test\"]}}]}}]}}'\n  exit 0\nfi\nprintf '%s\\n' \"$*\" >> \"$PROFILE_MARKER\"\nprintf '%s\\n' 'Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 2.00s'\nprintf '%s\\n' 'test result: ok. {passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out; finished in 1.00s'\nexit {exit}\n"
            ),
        )?;
        let workflow = temp.path().join("rust-test.yml");
        std::fs::write(
            &workflow,
            "jobs:\n  rust-test:\n    timeout-minutes: 4\n    steps:\n      - run: scripts/profile-rust-tests\n",
        )?;
        Ok(Self {
            temp,
            marker,
            bin_dir,
            workflow,
        })
    }

    fn run(
        &self,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        let path = format!("{}:{}", self.bin_dir.display(), std::env::var("PATH")?);
        let mut command = Command::new("python3");
        command
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests"))
            .args(["--root", env!("CARGO_MANIFEST_DIR"), "--workflow-file"])
            .arg(&self.workflow)
            .env("PATH", path)
            .env("PROFILE_MARKER", &self.marker);
        for (key, value) in environment {
            if *key == "EXTRA_ARGUMENT" {
                command.arg(value);
            } else {
                command.env(key, value);
            }
        }
        Ok(command.output()?)
    }
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, contents)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}
