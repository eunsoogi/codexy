use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::codegraph::{
    change_impact::{ImpactAnalysis, ImpactOptions, analyze_with_options},
    change_input::{ChangeScope, ChangeSet, collect},
};

use super::text_json;

pub(super) fn change_impact_result(root: &Path, args: &Value) -> Result<Value> {
    let (changes, impact) = collect_impact(root, args)?;
    text_json(&json!({
        "changeSet": changes,
        "impact": impact,
        "explanation": impact_explanation(&impact),
        "proofBoundary": proof_boundary()
    }))
}

pub(super) fn collect_impact(root: &Path, args: &Value) -> Result<(ChangeSet, ImpactAnalysis)> {
    let changes = collect(root, change_scope(args)?)?;
    let impact = analyze_with_options(root, &changes, impact_options(args)?)?;
    Ok((changes, impact))
}

fn change_scope(args: &Value) -> Result<ChangeScope> {
    match (
        optional_string(args, "base")?,
        optional_string(args, "head")?,
    ) {
        (Some(base), Some(head)) => Ok(ChangeScope::head_comparison(base, head)),
        (None, None) => Ok(ChangeScope::working_tree(bool_arg(
            args,
            "includeUntracked",
            true,
        )?)),
        _ => bail!("base and head must be provided together"),
    }
}

fn impact_options(args: &Value) -> Result<ImpactOptions> {
    let defaults = ImpactOptions::default();
    Ok(ImpactOptions {
        max_files: integer_arg(args, "maxFiles", defaults.max_files)?,
        max_paths: integer_arg(args, "maxPaths", defaults.max_paths)?,
    })
}

const fn impact_explanation(impact: &ImpactAnalysis) -> &'static str {
    if impact.partial {
        "Impact is partial or uncertain; inspect limits and unknown areas before choosing checks."
    } else {
        "Impact is complete within the configured Python and Rust analysis limits."
    }
}

pub(super) fn proof_boundary() -> Value {
    json!({
        "checksExecuted": false,
        "checksWaived": false,
        "completionDecided": false,
        "hostExposure": "unobserved"
    })
}

fn optional_string<'a>(args: &'a Value, name: &str) -> Result<Option<&'a str>> {
    match args.get(name) {
        None => Ok(None),
        Some(value) => Ok(Some(
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .with_context(|| format!("{name} must be a non-empty string"))?,
        )),
    }
}

fn bool_arg(args: &Value, name: &str, default: bool) -> Result<bool> {
    args.get(name)
        .map(|value| {
            value
                .as_bool()
                .with_context(|| format!("{name} must be a boolean"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn integer_arg(args: &Value, name: &str, default: usize) -> Result<usize> {
    args.get(name)
        .map(|value| {
            value_usize(value)
                .with_context(|| format!("{name} must be a finite non-negative integer"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn value_usize(value: &Value) -> Option<usize> {
    if let Some(number) = value.as_u64() {
        return usize::try_from(number).ok();
    }
    let number = value.as_f64()?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return None;
    }
    format!("{number:.0}").parse().ok()
}
