use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};

use super::{Case, ensure_field_order, field_names, record, scalar_field};

// These assertions retain field shapes, handoff coverage, and machine projections
// without pinning the public examples' prose to duplicated test strings.
pub(super) fn check_blind_read(cases: &[Case]) -> Result<()> {
    let handoff_reasons = [
        "FACT_CHECK",
        "CORRECTNESS_VERDICT",
        "MUTATION",
        "REFRESH",
        "AUTHOR_INTERVIEW",
        "HIDDEN_CRITERIA",
        "HISTORY",
        "VOTE",
        "STATUS_RECONSTRUCTION",
        "MULTIPLE_ARTIFACTS",
    ];
    let mut observed_reasons = HashSet::new();

    for case in cases {
        if case.classification == "POSITIVE" {
            ensure_field_order(
                case,
                &[
                    "artifact_bytes",
                    "intended_audience",
                    "intended_action",
                    "expected_purpose",
                    "expected_unresolved_reference",
                    "expected_action_blocker",
                ],
            )?;
        } else {
            ensure_field_order(
                case,
                &[
                    "artifact_bytes",
                    "intended_audience",
                    "intended_action",
                    "expected_result",
                ],
            )?;
        }
        for input in ["artifact_bytes", "intended_audience", "intended_action"] {
            ensure!(
                !scalar_field(case, input)?.is_empty(),
                "empty {input} in {}",
                case.id
            );
        }
        if case.classification == "POSITIVE" {
            ensure!(!scalar_field(case, "expected_purpose")?.is_empty());
            let reference = scalar_field(case, "expected_unresolved_reference")?;
            let blocker = scalar_field(case, "expected_action_blocker")?;
            ensure!(
                (reference == "<empty>") == (blocker == "<empty>"),
                "reference/blocker mismatch in {}",
                case.id
            );
        } else {
            let result = scalar_field(case, "expected_result")?;
            let reason = result
                .strip_prefix("HANDOFF_REQUIRED:")
                .with_context(|| format!("invalid handoff result in {}", case.id))?;
            ensure!(
                handoff_reasons.contains(&reason),
                "unknown reason in {}",
                case.id
            );
            observed_reasons.insert(reason.to_owned());
        }
    }

    ensure!(
        observed_reasons.len() == handoff_reasons.len(),
        "blind-read handoff coverage is incomplete"
    );
    Ok(())
}

pub(super) fn check_project_brief(cases: &[Case]) -> Result<()> {
    const MACHINE_FIELDS: [&str; 11] = [
        "scenario",
        "mode",
        "recorded",
        "expected_objective",
        "expected_owner",
        "expected_verified_phase",
        "expected_changes_since_touch",
        "expected_decision_required",
        "expected_evidence_handle",
        "expected_next_action",
        "expected_done_when",
    ];
    const HANDOFF_FIELDS: [&str; 2] = ["scenario", "expected_result"];
    const MACHINE_FIELDS_WITH_STALE_MEMORY: [&str; 12] = [
        "scenario",
        "mode",
        "recorded",
        "stale_memory",
        "expected_objective",
        "expected_owner",
        "expected_verified_phase",
        "expected_changes_since_touch",
        "expected_decision_required",
        "expected_evidence_handle",
        "expected_next_action",
        "expected_done_when",
    ];
    let output_fields = [
        "objective",
        "owner",
        "verified_phase",
        "changes_since_touch",
        "decision_required",
        "evidence_handle",
        "next_action",
        "done_when",
    ];

    let mut stale_memory_cases = 0;
    for case in cases {
        let has_stale_memory = field_names(case).contains(&"stale_memory");
        let expected_fields = if case.classification == "POSITIVE" {
            if has_stale_memory {
                stale_memory_cases += 1;
                &MACHINE_FIELDS_WITH_STALE_MEMORY[..]
            } else {
                &MACHINE_FIELDS[..]
            }
        } else {
            &HANDOFF_FIELDS[..]
        };
        ensure_field_order(case, expected_fields)?;
        ensure!(
            !scalar_field(case, "scenario")?.is_empty(),
            "empty scenario in {}",
            case.id
        );
        if case.classification == "POSITIVE" {
            let recorded = if scalar_field(case, "recorded")? == "none" {
                HashMap::new()
            } else {
                record(case, "recorded")?.1
            };
            ensure!(
                scalar_field(case, "mode")? == "machine",
                "mode changed in {}",
                case.id
            );
            if has_stale_memory {
                ensure!(!scalar_field(case, "stale_memory")?.is_empty());
            }
            for output in output_fields {
                let expected = scalar_field(case, &format!("expected_{output}"))?;
                let recorded_value = recorded
                    .get(output)
                    .map(String::as_str)
                    .unwrap_or("unavailable");
                ensure!(
                    expected == recorded_value,
                    "machine projection for {output} changed in {}",
                    case.id
                );
            }
        } else {
            ensure!(
                scalar_field(case, "expected_result")? == "HANDOFF_REQUIRED",
                "handoff result changed in {}",
                case.id
            );
        }
    }

    ensure!(stale_memory_cases == 1, "stale-memory coverage changed");

    Ok(())
}
