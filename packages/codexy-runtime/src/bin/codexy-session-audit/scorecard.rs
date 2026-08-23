use std::{collections::BTreeSet, fs, io::Read as _, path::Path};

use anyhow::{Context as _, Result, anyhow, bail};
use serde::Serialize;

use super::{MAX_INPUT_BYTES, is_safe_id};

#[path = "scorecard/schema.rs"]
pub(super) mod schema;
use schema::{
    Availability, Comparison, DecisionInput, EvidenceState, MeasureAvailability, Measurements,
    Phase, Scorecard, TaskClass, Thresholds,
};

const SCORECARD_SCHEMA: &str = "codexy.efficiency-scorecard.v1";
const CORPUS_ID: &str = "orchestration-1.5-baseline-v1";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Validation {
    valid: bool,
    corpus_id: String,
    comparison_count: usize,
    task_class_count: usize,
    owner_kind_count: usize,
    phase_count: usize,
    unavailable_measure_count: usize,
}

pub(super) fn validate_file(path: &Path) -> Result<Validation> {
    let input = fs::File::open(path).context("opening scorecard input")?;
    let mut bytes = Vec::new();
    input
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .context("reading scorecard input")?;
    if bytes.len() > MAX_INPUT_BYTES {
        bail!("scorecard input exceeds {MAX_INPUT_BYTES} bytes");
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow!("scorecard input does not match the closed schema"))?;
    if !schema::has_required_nullable_fields(&value) {
        bail!("scorecard input does not match the closed schema");
    }
    let scorecard: Scorecard = serde_json::from_value(value)
        .map_err(|_| anyhow!("scorecard input does not match the closed schema"))?;
    validate(scorecard)
}

fn validate(scorecard: Scorecard) -> Result<Validation> {
    if scorecard.schema != SCORECARD_SCHEMA || scorecard.corpus_id != CORPUS_ID {
        bail!("scorecard schema and corpus identity must match the packaged contract");
    }
    super::validate_scorecard_candidate(&scorecard.candidate)?;
    super::validate_scorecard_thresholds(&scorecard.thresholds)?;

    let mut ids = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let mut phases = BTreeSet::new();
    for comparison in &scorecard.comparisons {
        validate_comparison(comparison, &scorecard.measure_availability)?;
        if !ids.insert(comparison.id.as_str()) {
            bail!("scorecard comparison ids must be unique");
        }
        tasks.insert(comparison.task_class);
        owners.insert(comparison.owner.kind);
        phases.insert(comparison.phase);
    }
    if tasks.len() != 5 || owners.len() != 3 {
        bail!("scorecard must cover the representative corpus and every owner kind");
    }
    if scorecard.measure_availability.tool_output_bytes == Availability::Available
        && (!phases.contains(&Phase::Wait) || !phases.contains(&Phase::ToolOutput))
    {
        bail!("available tool-output evidence must separate wait and tool-output phases");
    }
    validate_decisions(
        &scorecard.decision_inputs,
        &scorecard.comparisons,
        &scorecard.measure_availability,
        &scorecard.thresholds,
        scorecard.candidate.installed_content_sha256.is_some(),
    )?;

    Ok(Validation {
        valid: true,
        corpus_id: scorecard.corpus_id,
        comparison_count: scorecard.comparisons.len(),
        task_class_count: tasks.len(),
        owner_kind_count: owners.len(),
        phase_count: phases.len(),
        unavailable_measure_count: super::availability_pairs(&scorecard.measure_availability)
            .iter()
            .filter(|(_, state)| *state == Availability::Unavailable)
            .count(),
    })
}

fn validate_comparison(comparison: &Comparison, availability: &MeasureAvailability) -> Result<()> {
    for id in [
        comparison.id.as_str(),
        comparison.optimization_id.as_str(),
        comparison.task_id.as_str(),
        comparison.model.as_str(),
        comparison.owner.id.as_str(),
    ] {
        if !is_safe_id(id) {
            bail!("scorecard identifiers must be bounded safe ids");
        }
    }
    if comparison.optimization_set.is_empty()
        || comparison.optimization_set.iter().any(|id| !is_safe_id(id))
        || comparison
            .optimization_set
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != comparison.optimization_set.len()
        || !comparison
            .optimization_set
            .contains(&comparison.optimization_id)
    {
        bail!("comparison optimization sets must be nonempty, safe, and contain the target");
    }
    if TaskClass::for_task_id(&comparison.task_id) != Some(comparison.task_class) {
        bail!("comparison task id and task class must match the representative corpus");
    }
    let (before, after, kind) = comparison.window.values();
    if before == 0 || before != after {
        bail!("{kind} windows must be positive and comparable");
    }
    validate_measurements(&comparison.before, availability)?;
    validate_measurements(&comparison.after, availability)
}

fn validate_measurements(
    measurements: &Measurements,
    availability: &MeasureAvailability,
) -> Result<()> {
    if measurements.accepted_runs > measurements.acceptance_runs
        || measurements.proof_complete_runs > measurements.acceptance_runs
    {
        bail!("scorecard quality aggregates must use consistent positive run counts");
    }
    if measurements
        .observed_cost_usd
        .is_some_and(|cost| !cost.is_finite() || cost < 0.0)
    {
        bail!("observed cost must be a finite non-negative value when available");
    }
    let values = [
        measurements.input_tokens.is_some(),
        measurements.wall_time_ms.is_some(),
        measurements.observed_cost_usd.is_some(),
        measurements.tool_input_bytes.is_some(),
        measurements.tool_output_bytes.is_some(),
        measurements.cache_input_tokens.is_some(),
    ];
    for ((name, state), present) in super::availability_pairs(availability).iter().zip(values) {
        match (state, present) {
            (Availability::Unavailable, true) => {
                bail!("unavailable measure must remain null: {name}")
            }
            (Availability::Available, false) => bail!("available measure must be recorded: {name}"),
            _ => {}
        }
    }
    Ok(())
}

fn validate_decisions(
    decisions: &[DecisionInput],
    comparisons: &[Comparison],
    availability: &MeasureAvailability,
    thresholds: &Thresholds,
    installed_content_available: bool,
) -> Result<()> {
    if decisions.is_empty() {
        bail!("scorecard must include per-optimization decision inputs");
    }
    let mut optimizations = BTreeSet::new();
    let mut covered = BTreeSet::new();
    let expected_unavailable = super::availability_pairs(availability)
        .into_iter()
        .filter_map(|(name, state)| (state == Availability::Unavailable).then_some(name))
        .collect::<BTreeSet<_>>();
    for decision in decisions {
        let selected = comparisons
            .iter()
            .filter(|comparison| decision.comparison_ids.contains(&comparison.id))
            .collect::<Vec<_>>();
        let expected_selected = comparisons
            .iter()
            .filter(|comparison| {
                comparison
                    .optimization_set
                    .contains(&decision.optimization_id)
            })
            .count();
        let unavailable = decision
            .unavailable_measures
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if !is_safe_id(&decision.optimization_id)
            || decision.comparison_ids.is_empty()
            || !optimizations.insert(decision.optimization_id.as_str())
            || selected.len() != decision.comparison_ids.len()
            || selected.len() != expected_selected
            || selected.iter().any(|comparison| {
                !comparison
                    .optimization_set
                    .contains(&decision.optimization_id)
            })
            || unavailable != expected_unavailable
        {
            bail!("decision inputs must bind one optimization to known comparisons");
        }
        if matches!(decision.evidence_state, EvidenceState::Observable)
            && (!installed_content_available
                || selected.iter().any(|comparison| {
                    comparison.before.acceptance_runs == 0 || comparison.after.acceptance_runs == 0
                }))
        {
            bail!("observable decision inputs require installed content and measured runs");
        }
        covered.extend(decision.comparison_ids.iter().map(String::as_str));
        if matches!(decision.evidence_state, EvidenceState::Observable) {
            super::validate_scorecard_outcomes(&selected, availability, thresholds)?;
        }
    }
    let comparison_ids = comparisons
        .iter()
        .map(|comparison| comparison.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected_optimizations = comparisons
        .iter()
        .flat_map(|comparison| comparison.optimization_set.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    if covered != comparison_ids || optimizations != expected_optimizations {
        bail!("decision inputs must completely cover comparisons and optimization sets");
    }
    Ok(())
}
