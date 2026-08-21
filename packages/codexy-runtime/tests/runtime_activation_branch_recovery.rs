#[path = "runtime_activation_branch_recovery/real.rs"]
mod real;

#[path = "runtime_activation_branch_recovery/fixture_matrix.rs"]
mod fixture_matrix;
#[path = "runtime_activation_branch_recovery/fixture_matrix_batch.rs"]
mod fixture_matrix_batch;

use std::process::Command;

use fixture_matrix::{Change, FixtureMatrix};

#[test]
fn existing_activation_branch_authenticates_exact_derived_tree_and_pr_state()
-> Result<(), Box<dyn std::error::Error>> {
    let matrix = FixtureMatrix::new()?;
    let exact = matrix.case(Change::Exact)?.run("OPEN")?;
    assert!(
        exact.status.success(),
        "exact activation failed: {}",
        String::from_utf8_lossy(&exact.stderr)
    );
    let batch = [
        matrix.batch_case("exact", Change::Exact, "OPEN", true)?,
        matrix.batch_case("wrapper-drift", Change::WrapperDrift, "OPEN", true)?,
        matrix.batch_case("bootstrap-drift", Change::BootstrapDrift, "OPEN", true)?,
        matrix.batch_case("release-contract-drift", Change::ReleaseContractDrift, "OPEN", true)?,
        matrix.batch_case("cargo-version-drift", Change::CargoVersionDrift, "OPEN", true)?,
        matrix.batch_case("extra", Change::Extra, "OPEN", true)?,
        matrix.batch_case("missing", Change::Missing, "OPEN", true)?,
        matrix.batch_case("closed", Change::Exact, "CLOSED", true)?,
        matrix.batch_case("ambiguous", Change::Exact, "OPEN\nOPEN", true)?,
        matrix.batch_case("test-mode", Change::Exact, "OPEN", false)?,
    ];
    let results = matrix.run_batch(&batch)?;
    assert_eq!(results.len(), batch.len(), "batch omitted a verifier state");
    assert!(results[0].success(), "batched exact activation failed");
    for (index, case) in batch.iter().enumerate().skip(1) {
        assert!(
            !results[index].success(),
            "{} unexpectedly passed:\nstdout:\n{}\nstderr:\n{}",
            case.name(),
            String::from_utf8_lossy(&results[index].stdout),
            String::from_utf8_lossy(&results[index].stderr),
        );
    }
    assert_diagnostic(&results[1], "activation branch differs from verified contract");
    assert_output(
        &results[2],
        "packages/codexy-runtime/src/version/bootstrap.rs",
    );
    assert_output(&results[3], ".agents/plugins/release-publish-contract.json");
    assert_output(&results[4], "packages/codexy-runtime/Cargo.toml");
    assert_diagnostic(&results[5], "activation branch differs from verified contract");
    assert_diagnostic(&results[6], ".agents/plugins/runtime-activation.json: No such file or directory");
    assert_diagnostic(&results[7], "activation branch has a closed or ambiguous pull request");
    assert_diagnostic(&results[8], "activation branch has a closed or ambiguous pull request");
    assert_diagnostic(&results[9], "test activator override requires CODEXY_TEST_MODE=1");
    assert_eq!(matrix.git_setup_starts(), 21, "seed plus mutation setup inventory");
    assert_eq!(matrix.batched_case_count(), 10, "all verifier states must remain");
    assert_eq!(matrix.verifier_starts(), 2, "single and batched verifier entrypoints");
    Ok(())
}

#[test]
fn existing_activation_branch_rejects_stale_component_manifest_after_reconstruction()
-> Result<(), Box<dyn std::error::Error>> {
    let matrix = FixtureMatrix::new()?;
    let stale = matrix.case(Change::StaleComponentManifest)?;
    let component_manifest = "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";
    let unchanged = Command::new("git")
        .args(["diff", "--quiet", "main...HEAD", "--", component_manifest])
        .current_dir(&stale.repo)
        .status()?;
    assert!(unchanged.success(), "fixture must leave the manifest unchanged in the branch diff");

    let output = stale.run("OPEN")?;
    assert!(
        !output.status.success(),
        "stale reconstructed component manifest unexpectedly passed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(diagnostics.contains(component_manifest), "missing stale manifest diagnostic: {diagnostics}");
    Ok(())
}

fn assert_diagnostic(result: &fixture_matrix_batch::BatchResult, expected: &str) {
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains(expected), "missing {expected:?} in batch stderr: {stderr}");
}

fn assert_output(result: &fixture_matrix_batch::BatchResult, expected: &str) {
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains(expected), "missing {expected:?} in batch stdout: {stdout}");
}
