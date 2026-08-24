use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

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

pub(super) fn route(policy: &Value, classification: &StableClassification) -> Result<Route> {
    let routing = policy
        .get("routing")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("canonical policy has no routing contract"))?;
    let (workflow, surfaces, risks, structured) = match classification {
        StableClassification::Legacy(workflow) => (workflow, &[][..], &[][..], false),
        StableClassification::Structured(value) => (
            &value.workflow,
            value.surfaces.as_slice(),
            value.risks.as_slice(),
            true,
        ),
    };
    label(workflow, "task classification")?;
    unique_labels(surfaces, "surface")?;
    unique_labels(risks, "risk")?;
    let tasks = strings(routing.get("task_classes"))?;
    let fail_closed_classes = strings(routing.get("fail_closed_classes"))?;
    let known_workflow = tasks.contains(workflow) || fail_closed_classes.contains(workflow);
    if !structured {
        ensure!(known_workflow, "handoff has an unknown task classification");
    }
    let known_surfaces = surfaces.iter().all(|surface| {
        routing["surface_names"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name.as_str() == Some(surface)))
    });
    let known_risks = risks.iter().all(|risk| {
        routing["risk_names"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name.as_str() == Some(risk)))
    });
    let fail_closed = if structured {
        !known_workflow
            || !known_surfaces
            || !known_risks
            || surfaces.is_empty()
            || !risks.is_empty()
            || fail_closed_classes.contains(workflow)
    } else {
        fail_closed_classes.contains(workflow)
    };
    let mut references = Vec::new();
    if fail_closed {
        add(
            &mut references,
            strings(routing.get("fallback_reference_route"))?,
        );
    } else if let Some(values) = routing["task_reference_routes"].get(workflow) {
        add(&mut references, strings(Some(values))?);
        for surface in surfaces {
            if let Some(values) = routing["surface_reference_routes"].get(surface) {
                add(&mut references, strings(Some(values))?);
            }
        }
    } else {
        ensure!(!structured, "handoff has an unknown task classification");
    }
    for risk in risks {
        if let Some(values) = routing["risk_reference_routes"].get(risk) {
            add(&mut references, strings(Some(values))?);
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

fn strings(value: Option<&Value>) -> Result<Vec<String>> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("policy route must be an array of strings"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("policy route contains a non-string"))
        })
        .collect()
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
