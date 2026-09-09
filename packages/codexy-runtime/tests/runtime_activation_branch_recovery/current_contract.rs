#[path = "current_contract/fixture.rs"]
mod fixture;

use std::process::Output;

use fixture::{WatcherCase, run_case, run_staged_case};

#[test]
fn verifier_tracks_current_and_older_package_versions_and_watcher_outputs()
-> Result<(), Box<dyn std::error::Error>> {
    for (base_version, watcher_case) in [
        ("1.7.0", WatcherCase::Expected),
        ("1.6.3", WatcherCase::Expected),
        ("1.7.0", WatcherCase::Missing),
    ] {
        let output = run_case(base_version, watcher_case)?;
        assert!(
            output.status.success(),
            "verifier rejected base {base_version}:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
    let staged = run_staged_case("1.6.3", WatcherCase::Expected)?;
    assert!(
        staged.status.success(),
        "verifier rejected staged activation:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&staged.stdout),
        String::from_utf8_lossy(&staged.stderr),
    );
    Ok(())
}

#[test]
fn verifier_rejects_tampered_watcher_and_unexpected_file()
-> Result<(), Box<dyn std::error::Error>> {
    let tampered = run_case("1.7.0", WatcherCase::Tampered)?;
    assert_failure_contains(&tampered, "plugins/codexy/mcp/codexy-mcp-watcher.sh");
    let unexpected = run_case("1.7.0", WatcherCase::Unexpected)?;
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
