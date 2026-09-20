use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::codegraph::check_selection::{
    CheckDefinition, CheckMapping, CheckMappings, CheckSelection, DependencyState, MappingKind,
    MappingOwner, SelectionOptions, recommend_with_options,
};

use super::impact::{collect_impact, proof_boundary};
use super::text_json;

pub(super) fn check_selection_result(root: &std::path::Path, args: &Value) -> Result<Value> {
    let (changes, impact) = collect_impact(root, args)?;
    let mappings = parse_mappings(args.get("mappings").context("mappings is required")?)?;
    let selection = recommend_with_options(&changes, &impact, &mappings, &selection_options(args)?);
    text_json(&serde_json::json!({
        "changeSet": changes,
        "impact": impact,
        "selection": selection,
        "explanation": selection_explanation(&selection),
        "proofBoundary": proof_boundary()
    }))
}

fn selection_options(args: &Value) -> Result<SelectionOptions> {
    let state = args
        .get("dependencyState")
        .map(|value| value.as_str().context("dependencyState must be a string"))
        .transpose()?
        .unwrap_or("unconfirmed");
    let dependency_state = match state {
        "confirmed" => DependencyState::Confirmed,
        "unconfirmed" => DependencyState::Unconfirmed {
            detail: match args.get("dependencyDetail") {
                None => "dependency state was not confirmed".to_owned(),
                Some(value) => value
                    .as_str()
                    .context("dependencyDetail must be a string")?
                    .to_owned(),
            },
        },
        _ => bail!("dependencyState must be confirmed or unconfirmed"),
    };
    Ok(SelectionOptions { dependency_state })
}

fn parse_mappings(value: &Value) -> Result<CheckMappings> {
    let checks = array_field(value, "checks")?
        .iter()
        .map(|check| {
            Ok(CheckDefinition {
                id: field_str(check, "id")?.to_owned(),
                command: field_str(check, "command")?.to_owned(),
                description: field_str(check, "description")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mappings = array_field(value, "mappings")?
        .iter()
        .map(parse_mapping)
        .collect::<Result<Vec<_>>>()?;
    Ok(CheckMappings::new(checks, mappings))
}

fn parse_mapping(mapping: &Value) -> Result<CheckMapping> {
    let owner = match field_str(mapping, "owner")? {
        "user" => MappingOwner::User,
        "repository" => MappingOwner::Repository,
        _ => bail!("mapping owner must be user or repository"),
    };
    let kind = match field_str(mapping, "kind")? {
        "path" => MappingKind::Path,
        "shared_configuration" => MappingKind::SharedConfiguration,
        "fixture" => MappingKind::Fixture,
        _ => bail!("mapping kind must be path, shared_configuration, or fixture"),
    };
    let check_ids = array_field_alias(mapping, "checkIds", "check_ids")?
        .iter()
        .map(|id| {
            id.as_str()
                .filter(|value| !value.is_empty())
                .context("mapping checkIds must contain strings")
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(str::to_owned)
        .collect();
    Ok(CheckMapping {
        owner,
        kind,
        pattern: field_str(mapping, "pattern")?.to_owned(),
        check_ids,
        reason: field_str(mapping, "reason")?.to_owned(),
    })
}

const fn selection_explanation(selection: &CheckSelection) -> &'static str {
    if selection.partial {
        "Recommendations are advisory and incomplete; review gaps, reasons, and limits manually."
    } else {
        "Recommendations are advisory data from explicit mappings; they do not execute or waive checks."
    }
}

fn array_field<'a>(value: &'a Value, name: &str) -> Result<&'a [Value]> {
    value
        .get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .with_context(|| format!("{name} must be an array"))
}

fn array_field_alias<'a>(value: &'a Value, name: &str, alias: &str) -> Result<&'a [Value]> {
    value
        .get(name)
        .or_else(|| value.get(alias))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .with_context(|| format!("{name} must be an array"))
}

fn field_str<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("{name} must be a non-empty string"))
}
