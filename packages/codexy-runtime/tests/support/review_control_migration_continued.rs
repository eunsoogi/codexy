use serde_json::Value;

use crate::support::TestResult;

use super::{fixtures, runner};

#[path = "review_control_direct_state.rs"]
mod direct_state;

#[path = "post_cap_review_graph.rs"]
mod graph;

pub(crate) fn run(profile: &str) -> TestResult<Value> {
    let temp = tempfile::tempdir()?;
    let repository = graph::SyntheticRepository::create(temp.path())?;
    let first_previous = fixtures::legacy_control(profile, 725, direct_state::SYNTHETIC_FULL_HEAD);
    let first_current = fixtures::migrated_control(
        profile,
        725,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
    );
    let (first_previous, previous_base, _) = repository.prepare(
        &first_previous,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let (first_current, _, current_base) = repository.prepare(
        &first_current,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_BASE,
    )?;
    let (first_result, first_state) = runner::run_transition_with_repository(
        temp.path(),
        None,
        &previous_base,
        &current_base,
        &first_previous,
        &first_current,
    )?;
    if !first_result.status.success() {
        return Err(format!(
            "initial migration failed: {}",
            String::from_utf8_lossy(&first_result.stderr)
        )
        .into());
    }
    let first_state = first_state.ok_or("initial migration did not write state")?;

    let next = fixtures::continued_control(
        profile,
        725,
        direct_state::SYNTHETIC_FULL_HEAD,
        direct_state::SYNTHETIC_DELTA_HEAD,
        direct_state::SYNTHETIC_CURRENT_HEAD,
    );
    let (next, _, next_base) = repository.prepare(
        &next,
        direct_state::SYNTHETIC_BASE,
        direct_state::SYNTHETIC_UPDATED_BASE,
    )?;
    let (next_result, next_state) = runner::run_transition_with_repository(
        temp.path(),
        Some(repository.path.as_path()),
        &current_base,
        &next_base,
        &first_state["reviewControl"],
        &next,
    )?;
    if !next_result.status.success() {
        return Err(format!(
            "continued migration failed: {}",
            String::from_utf8_lossy(&next_result.stderr)
        )
        .into());
    }
    next_state.ok_or_else(|| "continued migration did not write state".into())
}
