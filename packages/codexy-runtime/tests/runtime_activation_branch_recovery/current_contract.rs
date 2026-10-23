#[path = "current_contract/fixture.rs"]
mod fixture;

use std::process::Output;

use fixture::{CandidateCase, run_case, run_staged_case};

#[test]
// Activation must remove old base entrypoints from both current and previous package revisions.
fn verifier_tracks_retired_core_entrypoint_removal_for_current_and_older_packages()
-> Result<(), Box<dyn std::error::Error>> {
    for base_version in ["1.7.0", "1.6.3"] {
        let output = run_case(base_version, CandidateCase::NoRetiredPaths)?;
        assert!(
            output.status.success(),
            "verifier rejected base {base_version}:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    let staged = run_staged_case("1.6.3", CandidateCase::NoRetiredPaths)?;
    assert!(
        staged.status.success(),
        "verifier rejected staged activation:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&staged.stdout),
        String::from_utf8_lossy(&staged.stderr),
    );
    Ok(())
}

#[test]
fn verifier_rejects_stale_retired_core_launcher_and_unexpected_file()
-> Result<(), Box<dyn std::error::Error>> {
    let stale = run_case("1.7.0", CandidateCase::StaleLauncher)?;
    assert_failure_contains(&stale, "activation branch differs from verified contract");
    let unexpected = run_case("1.7.0", CandidateCase::Unexpected)?;
    assert_failure_contains(&unexpected, "activation branch differs from verified contract");
    Ok(())
}

fn assert_failure_contains(output: &Output, expected: &str) {
    assert!(!output.status.success(), "unexpected success");
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(diagnostics.contains(expected), "missing {expected:?}: {diagnostics}");
}
