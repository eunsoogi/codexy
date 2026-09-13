use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;

use super::super::routing_json;
use super::model::{
    Change, ChangePurpose, ENGINEERING, LegacyRequest, NON_ENGINEERING, ReproductionStatus,
    Request, V1_REQUEST_SCHEMA, V2_ENGINEERING, V2_REQUEST_SCHEMA, V2Request,
};

pub(super) fn parse_request(text: &str) -> Result<Request> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    match value.get("schema").and_then(Value::as_str) {
        Some(V1_REQUEST_SCHEMA) => {
            let request = serde_json::from_value::<LegacyRequest>(value)?;
            validate_v1(&request)?;
            Ok(Request::V1(request))
        }
        Some(V2_REQUEST_SCHEMA) => {
            let request = serde_json::from_value::<V2Request>(value)?;
            validate_v2(&request)?;
            Ok(Request::V2(request))
        }
        _ => bail!("TDD classification request has an unsupported schema"),
    }
}

fn validate_v1(request: &LegacyRequest) -> Result<()> {
    if request.schema != V1_REQUEST_SCHEMA {
        bail!("TDD classification request has an unsupported schema");
    }
    if request.boundaries.is_empty() || has_duplicates(&request.boundaries) {
        bail!("TDD classification request must contain unique boundaries");
    }
    if request.boundaries.iter().any(|boundary| {
        !ENGINEERING.contains(&boundary.as_str()) && !NON_ENGINEERING.contains(&boundary.as_str())
    }) {
        bail!("TDD classification request has an unrecognized boundary");
    }
    Ok(())
}

fn validate_v2(request: &V2Request) -> Result<()> {
    if request.schema != V2_REQUEST_SCHEMA {
        bail!("TDD classification request has an unsupported schema");
    }
    if request.boundaries.is_empty() {
        bail!("TDD classification v2 request must contain boundaries");
    }
    let mut ids = BTreeSet::new();
    for boundary in &request.boundaries {
        if boundary.id.trim().is_empty() || boundary.kind.trim().is_empty() {
            bail!("TDD classification v2 boundary id and kind must be non-empty");
        }
        if !ids.insert(&boundary.id) {
            bail!("TDD classification v2 boundary ids must be unique");
        }
        if !is_engineering(&boundary.kind) && !NON_ENGINEERING.contains(&boundary.kind.as_str()) {
            bail!("TDD classification v2 has an unrecognized boundary kind");
        }
        let engineering = is_engineering(&boundary.kind);
        if !engineering && (!boundary.risks.is_empty() || boundary.test_first_required) {
            bail!(
                "TDD classification v2 non-engineering boundaries cannot require engineering sequencing"
            );
        }
        if engineering && boundary.change.purpose == ChangePurpose::InstructionOnly {
            bail!(
                "TDD classification v2 instruction-only purpose cannot use an engineering boundary"
            );
        }
        validate_reproduction(&boundary.change)?;
        let mut risks = BTreeSet::new();
        if boundary.risks.iter().any(|risk| !risks.insert(risk)) {
            bail!("TDD classification v2 risks must be unique");
        }
    }
    Ok(())
}

fn validate_reproduction(change: &Change) -> Result<()> {
    match (&change.purpose, &change.reproduction) {
        (ChangePurpose::DefectRepair, None) => {
            bail!("TDD classification v2 defect repairs require reproduction status")
        }
        (ChangePurpose::DefectRepair, Some(reproduction)) => match reproduction.status {
            ReproductionStatus::Available
                if reproduction.reason.is_some() || reproduction.alternative.is_some() =>
            {
                bail!("TDD classification v2 available reproduction cannot include an alternative")
            }
            ReproductionStatus::Unavailable
                if !non_empty(&reproduction.reason) || !non_empty(&reproduction.alternative) =>
            {
                bail!(
                    "TDD classification v2 unavailable reproduction requires a reason and alternative"
                )
            }
            _ => Ok(()),
        },
        (_, Some(_)) => {
            bail!("TDD classification v2 reproduction is only valid for defect repairs")
        }
        _ => Ok(()),
    }
}

pub(super) fn is_engineering(kind: &str) -> bool {
    V2_ENGINEERING.contains(&kind)
}

fn non_empty(value: &Option<String>) -> bool {
    value
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn has_duplicates(values: &[String]) -> bool {
    let unique = values.iter().collect::<BTreeSet<_>>();
    unique.len() != values.len()
}
