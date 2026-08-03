#[path = "runtime_activation_branch_recovery/real.rs"]
mod real;

#[path = "runtime_activation_branch_recovery/fixture_matrix.rs"]
mod fixture_matrix;

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
    for change in [
        Change::WrapperDrift,
        Change::BootstrapDrift,
        Change::ReleaseContractDrift,
        Change::CargoVersionDrift,
        Change::Extra,
        Change::Missing,
    ] {
        assert!(
            !matrix.case(change)?.run("OPEN")?.status.success(),
            "{change:?} unexpectedly passed"
        );
    }
    assert!(!matrix.case(Change::Exact)?.run("CLOSED")?.status.success());
    assert!(!matrix.case(Change::Exact)?.run("OPEN\nOPEN")?.status.success());
    assert!(
        !matrix
            .case(Change::Exact)?
            .run_without_test_mode("OPEN")?
            .status
            .success()
    );
    assert_eq!(matrix.git_setup_starts(), 20, "seed plus mutation setup inventory");
    assert_eq!(matrix.verifier_starts(), 10, "all outer verifier E2Es must remain");
    Ok(())
}
