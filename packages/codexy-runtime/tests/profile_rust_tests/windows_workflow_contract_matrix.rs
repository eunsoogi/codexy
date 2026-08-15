use std::process::Command;

const PYTHON_MATRIX: &str = r#"
import contextlib
import importlib.util
import io
import json
import pathlib
import sys
import tempfile

script = pathlib.Path(sys.argv[1]).resolve()
cases_path = pathlib.Path(sys.argv[2]).resolve()
sys.path.insert(0, str(script.parent))
spec = importlib.util.spec_from_file_location("profile_rust_workflow", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
cases = json.loads(cases_path.read_text())
if len(cases) != 26:
    raise SystemExit(f"expected 26 workflow rows, found {len(cases)}")

with tempfile.TemporaryDirectory(prefix="codexy-workflow-matrix-") as directory:
    root = pathlib.Path(directory)
    for case in cases:
        workflow = root / f"{case['label']}.yml"
        workflow.write_text(case["workflow"])
        stderr = io.StringIO()
        try:
            with contextlib.redirect_stderr(stderr):
                module.enforce_workflow_contract(
                    workflow, 6, ("cargo", "test", "--locked", "--all-targets")
                )
            status = 0
        except SystemExit as error:
            status = error.code if isinstance(error.code, int) else 1
        expected_status = 0 if case["success"] else 1
        expected_stderr = case["stderr"] + ("\n" if case["stderr"] else "")
        if status != expected_status or stderr.getvalue() != expected_stderr:
            raise SystemExit(
                f"{case['label']}: status={status}, stderr={stderr.getvalue()!r}; "
                f"expected status={expected_status}, stderr={expected_stderr!r}"
            )
print(f"validated {len(cases)} workflow contract rows")
"#;

fn case(label: &str, workflow: String, success: bool, stderr: &str) -> serde_json::Value {
    serde_json::json!({
        "label": label,
        "workflow": workflow,
        "success": success,
        "stderr": if success { stderr } else { "Rust shard workflow has an invalid platform matrix" },
    })
}

fn matrix_cases() -> Vec<serde_json::Value> {
    let mut cases = Vec::new();
    for timeout in [10, 19, 21] {
        cases.push(case(
            &format!("timeout-{timeout}"),
            super::workflow(super::RUST_JOB, "windows-latest", timeout, super::WINDOWS_STEPS),
            false,
            "Windows Rust job must run the exact full workload once on windows-latest",
        ));
    }

    let rust_matrix = "    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        include: [one]\n    timeout-minutes: 6\n    steps:\n      - run: scripts/profile-rust-tests.py\n";
    for (label, rust_job, runner, steps, diagnostic) in [
        (
            "macos-runner",
            "    runs-on: macos-latest\n    timeout-minutes: 6\n    steps:\n      - run: scripts/profile-rust-tests.py\n",
            "windows-latest",
            super::WINDOWS_STEPS,
            "Rust test job must run once on ubuntu-latest without a matrix",
        ),
        (
            "matrix-runner",
            rust_matrix,
            "windows-latest",
            super::WINDOWS_STEPS,
            "Rust test job must run once on ubuntu-latest without a matrix",
        ),
        (
            "unapproved-windows-step",
            super::RUST_JOB,
            "windows-latest",
            "      - run: scripts/unapproved-windows-step.ps1\n      - run: python scripts/profile-rust-tests.py --windows\n",
            "Windows Rust job must run the exact full workload once on windows-latest",
        ),
        (
            "wrong-windows-runner",
            super::RUST_JOB,
            "ubuntu-latest",
            super::WINDOWS_STEPS,
            "Windows Rust job must run the exact full workload once on windows-latest",
        ),
    ] {
        cases.push(case(label, super::workflow(rust_job, runner, 20, steps), false, diagnostic));
    }

    let no_fetch = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests.py --windows\n";
    let no_toolchain = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests.py --windows\n";
    let unlocked_fetch = no_fetch.replace(
        "          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python",
        "          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python",
    );
    let extra_fetch = format!("{}      - run: cargo fetch --locked\n", super::WINDOWS_STEPS);
    let early_test = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          cargo test --locked --all-targets\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests.py --windows\n";
    for (label, steps) in [
        ("missing-toolchain-bootstrap", no_toolchain.to_string()),
        ("missing-cargo-fetch", no_fetch.to_string()),
        ("unlocked-cargo-fetch", unlocked_fetch),
        ("extra-cargo-fetch", extra_fetch),
        ("early-cargo-test", early_test.to_string()),
    ] {
        cases.push(case(
            label,
            super::workflow(super::RUST_JOB, "windows-latest", 20, &steps),
            false,
            "Windows Rust job must run the exact full workload once on windows-latest",
        ));
    }

    cases.push(case(
        "exact-sharded-workflow",
        std::fs::read_to_string(codexy_runtime::paths::repository_root().join(".github/workflows/rust-test.yml")).expect("read exact shard workflow"),
        true,
        "",
    ));
    for (label, bootstrap) in [
        ("empty-folded-run", "        run: >\n"),
        (
            "misaligned-folded-run",
            "        run: >\n        rustup toolchain install\n",
        ),
    ] {
        let steps = format!(
            "      - run: scripts/install-windows-test-prerequisites.ps1\n      - name: Bootstrap\n{bootstrap}      - run: python scripts/profile-rust-tests.py --windows\n"
        );
        cases.push(case(
            label,
            super::workflow(super::RUST_JOB, "windows-latest", 20, &steps),
            false,
            "Windows Rust job must run the exact full workload once on windows-latest",
        ));
    }

    for (position, controls) in [
        (
            "pre-profile",
            ["if: false", "continue-on-error: true", "\"if\": false", "'continue-on-error': true"],
        ),
        (
            "post-profile",
            ["if: false", "continue-on-error: true", "\"if\": false", "'continue-on-error': true"],
        ),
    ] {
        for (index, control) in controls.into_iter().enumerate() {
            let steps = if position == "pre-profile" {
                super::WINDOWS_STEPS.replacen(
                    "      - run: python scripts/profile-rust-tests.py --windows",
                    &format!("        {control}\n      - run: python scripts/profile-rust-tests.py --windows"),
                    1,
                )
            } else {
                super::WINDOWS_STEPS.replacen(
                    "      - run: python scripts/profile-rust-tests.py --windows",
                    &format!("      - run: python scripts/profile-rust-tests.py --windows\n        {control}"),
                    1,
                )
            };
            cases.push(case(
                &format!("{position}-control-{index}"),
                super::workflow(super::RUST_JOB, "windows-latest", 20, &steps),
                false,
                "Windows Rust job must run the exact full workload once on windows-latest",
            ));
        }
    }
    for (label, control) in [("job-if-false", "if: false"), ("job-continue-on-error", "continue-on-error: true")] {
        let source = super::workflow(super::RUST_JOB, "windows-latest", 20, super::WINDOWS_STEPS)
            .replacen(
                "    timeout-minutes: 20",
                &format!("    {control}\n    timeout-minutes: 20"),
                1,
            );
        cases.push(case(
            label,
            source,
            false,
            "Windows Rust job must run the exact full workload once on windows-latest",
        ));
    }

    let missing_failure_propagation = "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          cargo fetch --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile-rust-tests.py --windows\n";
    cases.push(case(
        "missing-rustup-failure-propagation",
        super::workflow(
            super::RUST_JOB,
            "windows-latest",
            20,
            missing_failure_propagation,
        ),
        false,
        "Windows Rust job must run the exact full workload once on windows-latest",
    ));
    assert_eq!(cases.len(), 26);
    cases
}

#[test]
fn workflow_contract_matrix_runs_in_one_python_process() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let cases = tempfile::NamedTempFile::new()?;
    std::fs::write(cases.path(), serde_json::to_vec(&matrix_cases())?)?;
    let output = Command::new("python3")
        .args(["-c", PYTHON_MATRIX])
        .arg(root.join("scripts/profile_rust_workflow.py"))
        .arg(cases.path())
        .output()?;
    assert!(
        output.status.success(),
        "workflow contract matrix failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "validated 26 workflow contract rows\n"
    );
    Ok(())
}
