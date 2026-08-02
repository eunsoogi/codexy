use crate::support::TestResult;
use std::path::Path;

const FULL_EXTERNAL: [(&str, usize); 13] = [
    ("tests/archive_fixture_nested_cargo.rs", 2),
    ("tests/profile_rust_tests/archive_inspection_receipts.rs", 2),
    ("tests/release_archive_gate/candidate.rs", 2),
    (
        "tests/release_archive_gate/candidate_projection_batch.rs",
        1,
    ),
    ("tests/runtime_activation_branch_recovery.rs", 1),
    ("tests/runtime_activation_branch_recovery/real.rs", 1),
    (
        "tests/runtime_publication_activation/activation_immutability.rs",
        1,
    ),
    ("tests/support/fixture_command_bindings.rs", 1),
    ("tests/support/fixture_command_controls.rs", 4),
    ("tests/support/fixture_probe.rs", 1),
    ("tests/support/release_archive.rs", 1),
    ("tests/support/wrapper.rs", 2),
    ("tests/sync_version_cli.rs", 1),
];

pub(super) fn assert_no_eligible_fixture_command_boundary() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let observed = FULL_EXTERNAL
        .iter()
        .map(|(relative, expected)| {
            let source =
                std::fs::read_to_string(root.join(relative)).map_err(|error| error.to_string())?;
            let actual = source.matches("FixtureCommand::new").count();
            (actual == *expected).then_some(actual).ok_or_else(|| {
                format!("{relative}: expected {expected} FixtureCommand boundaries, found {actual}")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total = observed.into_iter().sum::<usize>();
    assert_eq!(total, 20, "fixture command inventory drifted");
    println!(
        "stage11 benchmark persistent_command total_boundaries={total} eligible=0 savings_seconds=0.000000 reason=full-external"
    );
    Ok(())
}
