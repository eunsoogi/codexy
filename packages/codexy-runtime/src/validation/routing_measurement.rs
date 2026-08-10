use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "codexy.routing-evaluation-corpus.v1";
const RESULTS_SCHEMA: &str = "codexy.routing-evaluation-results.v1";
const EFFORTS: [&str; 3] = ["high", "xhigh", "max"];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema: String,
    corpus_id: String,
    tasks: Vec<Task>,
}
#[derive(Deserialize)]
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
struct Observation {
    task_id: String,
    prompt: String,
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
pub(super) fn check_canonical(plugin_root: &std::path::Path) -> Vec<String> {
    let corpus = plugin_root.join("skills/orchestration/references/routing-evaluation-corpus.json");
    let schema =
        plugin_root.join("skills/orchestration/references/routing-evaluation-results.schema.json");
    let mut errors = std::fs::read_to_string(&corpus)
        .map_err(|error| format!("{}: {error}", crate::paths::display_relative(&corpus)))
        .map_or_else(|error| vec![error], |text| corpus_diagnostics(&text));
    match std::fs::read_to_string(&schema)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        Some(schema) if closed_schema(&schema) => {}
        _ => errors.push(format!(
            "{} must be a closed routing-measurement JSON schema",
            crate::paths::display_relative(&schema)
        )),
    }
    errors
}
fn closed_schema(schema: &Value) -> bool {
    let Some(items) = schema
        .get("properties")
        .and_then(|value| value.get("results"))
        .and_then(|value| value.get("items"))
    else {
        return false;
    };
    let required = items
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    items.get("additionalProperties") == Some(&Value::Bool(false))
        && required
            == BTreeSet::from([
                "task_id",
                "prompt",
                "model",
                "thinking",
                "acceptance",
                "p0_p1_misses",
                "proof_complete",
                "repairs_retries",
                "tokens",
                "wall_time_ms",
                "observed_cost_usd",
            ])
}
pub(super) fn diagnostics(corpus: &str, results: &str) -> Vec<String> {
    let Ok(corpus) = serde_json::from_str::<Corpus>(corpus) else {
        return vec!["routing measurement corpus must be closed typed JSON".into()];
    };
    let mut errors = check_corpus(&corpus);
    let Ok(results) = serde_json::from_str::<Results>(results) else {
        errors.push("routing measurement results must be closed typed JSON".into());
        return errors;
    };
    if results.schema != RESULTS_SCHEMA || results.corpus_id != corpus.corpus_id {
        errors.push("routing measurement results must bind the frozen corpus identity".into());
    }
    let tasks = corpus
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task.prompt.as_str()))
        .collect::<BTreeMap<_, _>>();
    let expected = tasks
        .keys()
        .flat_map(|task| EFFORTS.map(move |effort| (*task, effort)))
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for result in &results.results {
        let key = (result.task_id.as_str(), result.thinking.as_str());
        if tasks.get(key.0) != Some(&result.prompt.as_str())
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
fn corpus_diagnostics(text: &str) -> Vec<String> {
    serde_json::from_str::<Corpus>(text).map_or_else(
        |_| vec!["routing measurement corpus must be closed typed JSON".into()],
        |corpus| check_corpus(&corpus),
    )
}
fn check_corpus(corpus: &Corpus) -> Vec<String> {
    let classes = corpus
        .tasks
        .iter()
        .map(|task| task.classification.as_str())
        .collect::<BTreeSet<_>>();
    let ids = corpus
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    if corpus.schema != CORPUS_SCHEMA
        || corpus.corpus_id.is_empty()
        || classes != BTreeSet::from(["ambiguous", "general", "simple"])
        || ids.len() != corpus.tasks.len()
        || corpus
            .tasks
            .iter()
            .any(|task| task.prompt.trim().is_empty() || task.acceptance_oracle.trim().is_empty())
    {
        vec!["routing measurement corpus must freeze unique simple, general, and ambiguous tasks with exact prompt identities".into()]
    } else {
        Vec::new()
    }
}

fn viable(observed: &BTreeMap<(&str, &str), &Observation>, effort: &str) -> bool {
    if effort == "high" {
        return false;
    }
    let high = EFFORTS[0];
    let records = observed
        .values()
        .copied()
        .filter(|result| result.thinking == effort)
        .collect::<Vec<_>>();
    let baseline = observed
        .values()
        .copied()
        .filter(|result| result.thinking == high)
        .collect::<Vec<_>>();
    let complete = records.iter().all(|result| {
        result.tokens.is_number()
            && result.wall_time_ms.is_number()
            && result.observed_cost_usd.is_number()
            && result.p0_p1_misses == 0
            && result.proof_complete
    });
    let comparable_baseline = baseline.iter().all(|result| {
        result.tokens.is_number()
            && result.wall_time_ms.is_number()
            && result.observed_cost_usd.is_number()
    });
    let acceptance = |records: &[&Observation]| {
        records
            .iter()
            .filter(|result| result.acceptance == "pass")
            .count() as f64
            / records.len() as f64
    };
    let repairs = |records: &[&Observation]| {
        records
            .iter()
            .map(|result| result.repairs_retries)
            .sum::<u32>()
    };
    complete
        && comparable_baseline
        && acceptance(&records) >= 0.95
        && (acceptance(&records) - acceptance(&baseline) >= 0.05
            || repairs(&baseline) > 0
                && repairs(&records) as f64 <= repairs(&baseline) as f64 * 0.8)
        && median(&records, |result| &result.wall_time_ms)
            <= median(&baseline, |result| &result.wall_time_ms) * 1.5
        && median(&records, |result| &result.observed_cost_usd)
            <= median(&baseline, |result| &result.observed_cost_usd) * 1.5
}

fn metric(value: &Value, integer: bool) -> bool {
    value.is_null()
        || if integer {
            value.as_u64().is_some()
        } else {
            value.as_f64().is_some_and(|value| value >= 0.0)
        }
}
fn median(records: &[&Observation], field: impl Fn(&Observation) -> &Value) -> f64 {
    let mut values = records
        .iter()
        .filter_map(|result| field(result).as_f64())
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
