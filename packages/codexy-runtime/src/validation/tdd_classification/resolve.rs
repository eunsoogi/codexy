use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::path::Path;

use super::model::{
    BoundaryRequest, ChangePurpose, ReproductionStatus, Request, Risk, TddMode, V2_RESULT_SCHEMA,
};
use super::validation::{is_engineering, parse_request};

pub(super) fn resolve(_plugin_root: &Path, text: &str) -> Result<Value> {
    match parse_request(text)? {
        Request::V1(request) => resolve_v1(request),
        Request::V2(request) => resolve_v2(request),
    }
}

fn resolve_v1(request: super::model::LegacyRequest) -> Result<Value> {
    let engineering = request
        .boundaries
        .iter()
        .filter(|boundary| super::model::ENGINEERING.contains(&boundary.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let proportional = request
        .boundaries
        .iter()
        .filter(|boundary| super::model::NON_ENGINEERING.contains(&boundary.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let classification = classification(engineering.is_empty(), proportional.is_empty())?;
    Ok(json!({
        "classification": classification,
        "engineering_tdd_required": !engineering.is_empty(),
        "tdd_boundaries": engineering,
        "proportional_proof_boundaries": proportional,
    }))
}

fn resolve_v2(request: super::model::V2Request) -> Result<Value> {
    let engineering = request
        .boundaries
        .iter()
        .filter(|boundary| is_engineering(&boundary.kind))
        .map(|boundary| boundary.kind.clone())
        .collect::<Vec<_>>();
    let proportional = request
        .boundaries
        .iter()
        .filter(|boundary| !is_engineering(&boundary.kind))
        .map(|boundary| boundary.kind.clone())
        .collect::<Vec<_>>();
    let modes = request
        .boundaries
        .iter()
        .map(|boundary| mode_for(boundary, is_engineering(&boundary.kind)))
        .collect::<Vec<_>>();
    let obligations = request
        .boundaries
        .iter()
        .map(|boundary| obligation(boundary, is_engineering(&boundary.kind)))
        .collect::<Vec<_>>();
    let classification = classification(engineering.is_empty(), proportional.is_empty())?;
    Ok(json!({
        "schema": V2_RESULT_SCHEMA,
        "classification": classification,
        "engineering_tests_required": !engineering.is_empty(),
        "tdd_mode": aggregate_mode(&modes).as_str(),
        "tdd_boundaries": engineering,
        "proportional_proof_boundaries": proportional,
        "boundary_obligations": obligations,
    }))
}

fn obligation(boundary: &BoundaryRequest, engineering: bool) -> Value {
    let mode = mode_for(boundary, engineering);
    let proof_obligations = if engineering {
        vec!["requirement_linked_behavioral_verification"]
    } else {
        vec!["proportional_structural_or_behavioral_proof"]
    };
    json!({
        "id": boundary.id,
        "kind": boundary.kind,
        "engineering_tests_required": engineering,
        "tdd_mode": mode.as_str(),
        "pre_change_obligations": pre_change_obligations(boundary, mode),
        "proof_obligations": proof_obligations,
    })
}

fn mode_for(boundary: &BoundaryRequest, engineering: bool) -> TddMode {
    if !engineering {
        return TddMode::NotApplicable;
    }
    if boundary.test_first_required {
        return TddMode::Required;
    }
    match boundary
        .change
        .reproduction
        .as_ref()
        .map(|item| item.status)
    {
        Some(ReproductionStatus::Available) => TddMode::Required,
        _ => TddMode::Optional,
    }
}

fn pre_change_obligations(boundary: &BoundaryRequest, mode: TddMode) -> Vec<&'static str> {
    let mut obligations = boundary
        .risks
        .iter()
        .map(|risk| match risk {
            Risk::Permission => "permission_invariants_before_change",
            Risk::DestructiveState => "destructive_state_invariants_before_change",
            Risk::Recovery => "recovery_invariants_before_change",
        })
        .collect::<Vec<_>>();
    match boundary
        .change
        .reproduction
        .as_ref()
        .map(|item| item.status)
    {
        Some(ReproductionStatus::Available) => obligations.push("faithful_red_before_fix"),
        Some(ReproductionStatus::Unavailable) => {
            obligations.push("justified_alternative_before_change")
        }
        None if boundary.change.purpose == ChangePurpose::BehaviorPreservingRefactor => {
            obligations.push("green_or_characterization_baseline")
        }
        None => {}
    }
    if mode == TddMode::Required && boundary.test_first_required {
        obligations.push("test_first_sequence");
    }
    obligations
}

fn aggregate_mode(modes: &[TddMode]) -> TddMode {
    if modes.contains(&TddMode::Required) {
        TddMode::Required
    } else if modes.contains(&TddMode::Optional) {
        TddMode::Optional
    } else {
        TddMode::NotApplicable
    }
}

fn classification(engineering_empty: bool, proportional_empty: bool) -> Result<&'static str> {
    match (engineering_empty, proportional_empty) {
        (false, true) => Ok("engineering"),
        (true, false) => Ok("non_engineering"),
        (false, false) => Ok("mixed"),
        (true, true) => bail!("TDD classification request has no recognized boundary"),
    }
}
