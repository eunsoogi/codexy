#[path = "stage11_harness_benchmark/fixture_session.rs"]
mod fixture_session;
#[path = "stage11_harness_benchmark/persistent_command.rs"]
mod persistent_command;

use crate::support::TestResult;

#[test]
fn stage11_harness_architecture_benchmark_preserves_private_fixture_semantics() -> TestResult {
    persistent_command::assert_no_eligible_fixture_command_boundary()?;
    let measurement = fixture_session::measure_resettable_private_sessions()?;
    println!(
        "stage11 benchmark fixture_session baseline_seconds={:.6} candidate_seconds={:.6} \
         baseline_files={} baseline_bytes={} candidate_files={} candidate_bytes={} cases={}",
        measurement.baseline_seconds,
        measurement.candidate_seconds,
        measurement.baseline_files,
        measurement.baseline_bytes,
        measurement.candidate_files,
        measurement.candidate_bytes,
        measurement.cases,
    );
    Ok(())
}
