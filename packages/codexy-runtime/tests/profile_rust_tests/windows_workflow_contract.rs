use std::process::Command;

use super::super::GateFixture;

const RUST_JOB: &str =
    "    runs-on: ubuntu-latest\n    timeout-minutes: 6\n    steps:\n      - run: scripts/profile_rust_tests.py\n";
const WINDOWS_STEPS: &str =
    "      - run: scripts/install-windows-test-prerequisites.ps1\n      - run: |\n          rustup toolchain install\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n          cargo fetch --manifest-path packages/codexy-runtime/Cargo.toml --locked\n          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n      - run: python scripts/profile_rust_tests.py --windows\n";
const SHARD_MATRIX: &str = "        shard: [\n          support,\n          agent,\n          child,\n          orchestration,\n          governance,\n          system,\n          archive,\n        ]\n";

fn workflow(rust_job: &str, windows_runner: &str, timeout: u8, steps: &str) -> String {
    format!(
        "jobs:\n  rust-test:\n{rust_job}  windows-rust-test:\n    runs-on: {windows_runner}\n    timeout-minutes: {timeout}\n    steps:\n{steps}"
    )
}

#[path = "windows_workflow_contract_matrix.rs"]
mod matrix;

#[test]
fn rust_workflow_runs_the_full_suite_natively_on_windows() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/rust-test.yml"),
    )
    .expect("read Rust workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "rust-workflow-windows-suite",
        &[
            "windows-rust-test:",
            "name: Rust shard (Windows, ${{ matrix.shard }})",
            "runs-on: windows-latest",
            "timeout-minutes: 20",
            "max-parallel: 7",
            "ref: ${{ github.event.pull_request.head.sha }}",
            "run: python scripts/profile_rust_tests.py --windows --shard ${{ matrix.shard }}",
        ],
    );
    assert_eq!(workflow.matches("windows-rust-profile").count(), 0);
    assert_eq!(workflow.matches("scripts/profile_rust_tests.py").count(), 3);
    assert_eq!(
        workflow
            .matches("ref: ${{ github.event.pull_request.head.sha }}")
            .count(),
        3
    );

    for invalid_workflow in [
        workflow.replacen("max-parallel: 7", "max-parallel: 6", 1),
        workflow.replacen(SHARD_MATRIX, "        shard: [support] # [support, agent, child, orchestration, governance, system, archive]\n", 1),
        workflow.replacen("          persist-credentials: false\n", "", 1).replacen("          merge-multiple: true\n", "          merge-multiple: true\n          persist-credentials: false\n", 1),
        workflow.replacen("      - if: always()\n        uses: actions/upload-artifact@v7", "      - uses: actions/upload-artifact@v7\n        # if: always()", 1),
        workflow.replacen(SHARD_MATRIX, "        include:\n          - shard: support\n        # shard: [support, agent, child, orchestration, governance, system, archive]\n", 1),
        workflow.replacen("      - uses: actions/checkout@v7\n        with:\n          ref: ${{ github.event.pull_request.head.sha }}\n          fetch-depth: 0\n          persist-credentials: false\n", "", 1),
        workflow.replacen("          ref: ${{ github.event.pull_request.head.sha }}\n", "", 1),
        workflow.replacen("          ref: ${{ github.event.pull_request.head.sha }}", "          ref: ${{ github.sha }}", 1),
        workflow.replacen("          ref: ${{ github.event.pull_request.head.sha }}\n", "          # ref: ${{ github.event.pull_request.head.sha }}\n", 1),
        workflow.replacen("          ref: ${{ github.event.pull_request.head.sha }}\n", "", 2).replacen("          ref: ${{ github.event.pull_request.head.sha }}\n", "          ref: refs/pull/516/merge\n", 1),
        workflow.replacen("          ref: ${{ github.event.pull_request.head.sha }}\n          fetch-depth: 0\n", "          fetch-depth: 0\n      - run: echo '${{ github.event.pull_request.head.sha }}'\n", 1),
        workflow.replacen("      - shell: pwsh\n        run: scripts/install-windows-test-prerequisites.ps1\n", "", 1),
        workflow.replacen("      - shell: pwsh\n        run: rustup toolchain install; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo fetch --manifest-path packages/codexy-runtime/Cargo.toml --locked; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }\n", "", 1),
        workflow.replacen("    steps:\n", "    env:\n      RUST_TEST_THREADS: 1\n    steps:\n", 1),
        workflow.replacen(SHARD_MATRIX, &format!("{SHARD_MATRIX}        extra: rejected\n"), 1),
        workflow.replacen("      - run: scripts/profile_rust_tests.py --shard", "      - env:\n          CARGO_PROFILE_TEST_INCREMENTAL: true\n        run: scripts/profile_rust_tests.py --shard", 1),
        workflow.replacen("      - uses: actions/download-artifact@v8\n", "      - uses: actions/download-artifact@v8\n        if: always()\n", 1),
        workflow.replacen("permissions:\n", "env:\n  RUST_TEST_THREADS: 1\npermissions:\n", 1),
        workflow.replacen("permissions:\n", "env:\n  CARGO_PROFILE_TEST_INCREMENTAL: true\npermissions:\n", 1),
    ] {
        let invalid = tempfile::NamedTempFile::new().expect("temporary workflow");
        std::fs::write(invalid.path(), invalid_workflow).expect("write invalid workflow");
        let output = Command::new("python3")
        .args(["-c", "import pathlib, sys; sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent)); from profile_rust_workflow import enforce_workflow_contract; enforce_workflow_contract(pathlib.Path(sys.argv[2]), 6, ('cargo', 'test', '--locked', '--all-targets'))"])
        .arg(codexy_runtime::paths::repository_root().join("scripts/profile_rust_workflow.py"))
        .arg(invalid.path())
            .output()
            .expect("run workflow validator");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "Rust shard workflow has an invalid platform matrix\n");
    }
}

#[test]
fn rust_workflow_overwrites_fixed_name_receipts_for_reruns() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join(".github/workflows/rust-test.yml"),
    )
    .expect("read Rust workflow");
    let upload_marker =
        "      - if: always()\n        uses: actions/upload-artifact@v7\n        with:\n";
    let uploads: Vec<_> = workflow
        .split(upload_marker)
        .skip(1)
        .map(|tail| tail.split("\n\n").next().expect("upload block"))
        .collect();

    assert_eq!(uploads.len(), 2, "expected one fixed-name producer per platform");
    for (upload, platform) in uploads.iter().zip(["posix", "windows"]) {
        assert!(
            upload.contains("          overwrite: true"),
            "{platform} producer must overwrite its fixed-name retry artifact: {upload}"
        );
        assert!(
            upload.contains(&format!("          name: rust-receipt-{platform}-${{{{ matrix.shard }}}}")),
            "{platform} producer must keep its fixed receipt name: {upload}"
        );
    }
}

#[test]
fn gate_rejects_a_fixed_name_upload_without_overwrite() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let workflow = std::fs::read_to_string(&fixture.workflow)?;
    let missing_overwrite = workflow.replace("          overwrite: true\n", "");
    assert_eq!(missing_overwrite.matches("actions/upload-artifact@v7").count(), 2);
    std::fs::write(&fixture.workflow, missing_overwrite)?;

    let output = fixture.run(&[])?;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(String::from_utf8(output.stderr)?.contains("invalid platform matrix"));
    Ok(())
}

#[test]
fn gate_retains_the_exact_native_windows_positive() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(
        &fixture.workflow,
        workflow(RUST_JOB, "windows-latest", 20, WINDOWS_STEPS),
    )?;
    assert!(!fixture.run(&[])?.status.success());
    Ok(())
}

#[test]
fn gate_retains_the_missing_windows_job_negative() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    std::fs::write(&fixture.workflow, format!("jobs:\n  rust-test:\n{RUST_JOB}"))?;
    assert!(!fixture.run_without_required_windows_job(&[])?.status.success());
    Ok(())
}
