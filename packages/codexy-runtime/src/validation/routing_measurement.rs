use anyhow::{Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{routing_json, routing_measurement_schema::is_closed};

mod viability;
use viability::viable;

const CORPUS_SCHEMA: &str = "codexy.routing-evaluation-corpus.v1";
const RESULTS_SCHEMA: &str = "codexy.routing-evaluation-results.v1";
const EFFORTS: [&str; 3] = ["high", "xhigh", "max"];
const CORPUS_PATH: &str = "skills/orchestration/references/routing-evaluation-corpus.json";
const SCHEMA_PATH: &str = "skills/orchestration/references/routing-evaluation-results.schema.json";

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: String,
    corpus_id: String,
    tasks: Vec<Task>,
}

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Task {
    id: String,
    classification: String,
    prompt: String,
    acceptance_oracle: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Results {
    schema: String,
    corpus_id: String,
    selected_effort: String,
    results: Vec<Observation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    task_id: String,
    prompt: String,
    acceptance_oracle: String,
    model: String,
    thinking: String,
    acceptance: String,
    p0_p1_misses: u32,
    proof_complete: bool,
    repairs_retries: u32,
    tokens: Value,
    wall_time_ms: Value,
    observed_cost_usd: Value,
}

pub(super) fn check_canonical(plugin_root: &Path, results_path: &Path) -> Vec<String> {
    let corpus_path = plugin_root.join(CORPUS_PATH);
    let schema_path = plugin_root.join(SCHEMA_PATH);
    let corpus = read_corpus(&corpus_path);
    let mut errors = corpus
        .as_ref()
        .map_or_else(|error| vec![error.to_string()], validate_corpus);
    match std::fs::read_to_string(&schema_path)
        .ok()
        .and_then(|text| routing_json::parse(&text).ok())
    {
        Some(schema) if is_closed(&schema) => {}
        _ => errors.push(format!(
            "{} must be a closed routing-measurement JSON schema",
            crate::paths::display_relative(&schema_path)
        )),
    }
    match (corpus, std::fs::read_to_string(results_path)) {
        (Ok(corpus), Ok(results)) => errors.extend(diagnostics_for(&corpus, &results)),
        (_, Err(error)) => errors.push(format!(
            "{}: {error}",
            crate::paths::display_relative(results_path)
        )),
        _ => {}
    }
    errors
}

pub(super) fn diagnostics(plugin_root: &Path, corpus: &str, results: &str) -> Vec<String> {
    let canonical = read_corpus(&plugin_root.join(CORPUS_PATH));
    let input = parse_corpus(corpus);
    match (canonical, input) {
        (Ok(canonical), Ok(input)) if input == canonical => diagnostics_for(&canonical, results),
        (Ok(_), Ok(_)) => {
            vec!["routing measurement corpus must match the packaged frozen corpus".into()]
        }
        (Err(error), _) | (_, Err(error)) => vec![error.to_string()],
    }
}

pub(super) fn selected_effort(plugin_root: &Path, corpus: &str, results: &str) -> Result<String> {
    let errors = diagnostics(plugin_root, corpus, results);
    if errors.is_empty() {
        parse_results(results).map(|results| results.selected_effort)
    } else {
        bail!(errors.join("; "))
    }
}

fn read_corpus(path: &Path) -> Result<Corpus> {
    parse_corpus(&std::fs::read_to_string(path)?)
}

fn parse_corpus(text: &str) -> Result<Corpus> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    let corpus = serde_json::from_value::<Corpus>(value)?;
    if validate_corpus(&corpus).is_empty() {
        Ok(corpus)
    } else {
        bail!("routing measurement corpus must be closed typed JSON")
    }
}

fn parse_results(text: &str) -> Result<Results> {
    let value = routing_json::parse(text).map_err(anyhow::Error::msg)?;
    Ok(serde_json::from_value(value)?)
}

fn validate_corpus(corpus: &Corpus) -> Vec<String> {
    let ids = corpus
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    if corpus.schema != CORPUS_SCHEMA
        || corpus.corpus_id.trim().is_empty()
        || corpus.tasks.is_empty()
        || ids.len() != corpus.tasks.len()
        || corpus.tasks.iter().any(|task| {
            task.classification.is_empty()
                || task.prompt.is_empty()
                || task.acceptance_oracle.is_empty()
        })
    {
        vec!["routing measurement corpus must be closed typed JSON".into()]
    } else {
        Vec::new()
    }
}

fn diagnostics_for(corpus: &Corpus, results: &str) -> Vec<String> {
    let Ok(results) = parse_results(results) else {
        return vec!["routing measurement results must be closed typed JSON".into()];
    };
    let mut errors = Vec::new();
    if results.schema != RESULTS_SCHEMA || results.corpus_id != corpus.corpus_id {
        errors.push("routing measurement results must bind the frozen corpus identity".into());
    }
    let tasks = corpus
        .tasks
        .iter()
        .map(|task| {
            (
                task.id.as_str(),
                (task.prompt.as_str(), task.acceptance_oracle.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = tasks
        .keys()
        .flat_map(|task| EFFORTS.map(move |effort| (*task, effort)))
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for result in &results.results {
        let key = (result.task_id.as_str(), result.thinking.as_str());
        if tasks.get(key.0) != Some(&(result.prompt.as_str(), result.acceptance_oracle.as_str()))
            || result.model != "gpt-5.6-terra"
            || !EFFORTS.contains(&key.1)
            || !matches!(result.acceptance.as_str(), "pass" | "fail")
            || !metric(&result.tokens, true)
            || !metric(&result.wall_time_ms, true)
            || !metric(&result.observed_cost_usd, false)
            || observed.insert(key, result).is_some()
        {
            errors.push("routing measurement results must contain one closed Terra observation for every frozen task and effort".into());
            break;
        }
    }
    if observed.keys().copied().collect::<BTreeSet<_>>() != expected {
        errors
            .push("incomplete or non-comparable routing measurement must retain Terra/high".into());
        return errors;
    }
    let viable = EFFORTS.map(|effort| viable(&observed, effort));
    let required = if viable[1] {
        "xhigh"
    } else if viable[2] {
        "max"
    } else {
        "high"
    };
    if results.selected_effort != required {
        errors.push(format!("routing measurement must retain Terra/high or select the lowest sufficient effort; expected {required}"));
    }
    errors
}

fn metric(value: &Value, integer: bool) -> bool {
    value.is_null()
        || if integer {
            value.as_u64().is_some()
        } else {
            value.as_f64().is_some_and(|number| number >= 0.0)
        }
}
