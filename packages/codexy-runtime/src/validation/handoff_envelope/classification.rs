use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod routes;
use routes::{
    fail_closed_class, fallback_route, known_risk, known_surface, known_workflow, risk_route,
    surface_route, task_route,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum StableClassification {
    Legacy(String),
    Structured(StructuredClassification),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredClassification {
    pub workflow: String,
    pub surfaces: Vec<String>,
    pub risks: Vec<String>,
}

pub(super) struct Route {
    pub references: Vec<String>,
    pub fail_closed: bool,
}

impl From<&str> for StableClassification {
    fn from(value: &str) -> Self {
        Self::Legacy(value.to_owned())
    }
}

impl From<String> for StableClassification {
    fn from(value: String) -> Self {
        Self::Legacy(value)
    }
}

pub(super) fn route(classification: &StableClassification) -> Result<Route> {
    let (workflow, surfaces, risks, structured) = match classification {
        StableClassification::Legacy(workflow) => (workflow.as_str(), &[][..], &[][..], false),
        StableClassification::Structured(value) => (
            value.workflow.as_str(),
            value.surfaces.as_slice(),
            value.risks.as_slice(),
            true,
        ),
    };
    label(workflow, "task classification")?;
    unique_labels(surfaces, "surface")?;
    unique_labels(risks, "risk")?;
    let known_workflow = known_workflow(workflow);
    if !structured {
        ensure!(known_workflow, "handoff has an unknown task classification");
    }
    let known_surfaces = surfaces.iter().map(String::as_str).all(known_surface);
    let known_risks = risks.iter().map(String::as_str).all(known_risk);
    let fail_closed = if structured {
        !known_workflow
            || !known_surfaces
            || !known_risks
            || surfaces.is_empty()
            || !risks.is_empty()
            || fail_closed_class(workflow)
    } else {
        fail_closed_class(workflow)
    };
    let mut references = Vec::new();
    if fail_closed {
        add(&mut references, fallback_route());
    } else if let Some(values) = task_route(workflow) {
        add(&mut references, values);
        for surface in surfaces {
            if let Some(values) = surface_route(surface) {
                add(&mut references, values);
            }
        }
    } else {
        ensure!(!structured, "handoff has an unknown task classification");
    }
    for risk in risks {
        if let Some(values) = risk_route(risk) {
            add(&mut references, values);
        }
    }
    Ok(Route {
        references,
        fail_closed,
    })
}

fn add(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
}

fn label(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control),
        "{field} must be bounded, non-empty, and free of control characters"
    );
    Ok(())
}

fn unique_labels(values: &[String], field: &str) -> Result<()> {
    ensure!(
        values.iter().all(|value| label(value, field).is_ok()),
        "{field} contains an invalid value"
    );
    ensure!(
        values.iter().collect::<BTreeSet<_>>().len() == values.len(),
        "{field} contains duplicate values"
    );
    Ok(())
}
