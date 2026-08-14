use std::path::Path;

use serde_json::{Map, Value};

use super::Disposition;

pub(super) fn disposition(path: &Path, text: &str, base: Disposition) -> Disposition {
    if base != Disposition::Maintained {
        return base;
    }
    if manifested(path, text)
        || serde_json::from_str::<Value>(text)
            .ok()
            .is_some_and(|value| exact_json(&value))
    {
        Disposition::ExactFixture
    } else {
        Disposition::Maintained
    }
}

fn manifested(path: &Path, text: &str) -> bool {
    let Some(marker) = text
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("// codexy-exact-fixture-file: "))
    else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(include_str!("exact_fixture_manifest.json"))
    else {
        return false;
    };
    manifest.get("schema").and_then(Value::as_str) == Some("codexy.exact-fixture-manifest.v1")
        && manifest
            .get("fixtures")
            .and_then(Value::as_array)
            .is_some_and(|fixtures| {
                fixtures.iter().any(|fixture| {
                    fixture.get("path").and_then(Value::as_str) == path.to_str()
                        && fixture.get("marker").and_then(Value::as_str) == Some(marker)
                })
            })
}

fn exact_json(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    corpus(object) || results(object) || schema(object) || components(object)
}

fn corpus(object: &Map<String, Value>) -> bool {
    object.get("schema").and_then(Value::as_str) == Some("codexy.routing-evaluation-corpus.v1")
        && nonempty(object, "corpus_id")
        && object
            .get("tasks")
            .and_then(Value::as_array)
            .is_some_and(|tasks| {
                !tasks.is_empty()
                    && tasks.iter().all(|task| {
                        task.as_object().is_some_and(|task| {
                            ["id", "classification", "prompt", "acceptance_oracle"]
                                .iter()
                                .all(|key| nonempty(task, key))
                        })
                    })
            })
}

fn results(object: &Map<String, Value>) -> bool {
    object.get("schema").and_then(Value::as_str) == Some("codexy.routing-evaluation-results.v1")
        && nonempty(object, "corpus_id")
        && matches!(
            object.get("selected_effort").and_then(Value::as_str),
            Some("high" | "xhigh" | "max")
        )
        && object
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| !results.is_empty() && results.iter().all(result))
}

fn result(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    [
        "task_id",
        "prompt",
        "acceptance_oracle",
        "model",
        "thinking",
        "acceptance",
    ]
    .iter()
    .all(|key| nonempty(object, key))
        && object.get("model").and_then(Value::as_str) == Some("gpt-5.6-terra")
        && matches!(
            object.get("thinking").and_then(Value::as_str),
            Some("high" | "xhigh" | "max")
        )
        && matches!(
            object.get("acceptance").and_then(Value::as_str),
            Some("pass" | "fail")
        )
        && object.get("p0_p1_misses").and_then(Value::as_u64).is_some()
        && object
            .get("proof_complete")
            .and_then(Value::as_bool)
            .is_some()
        && object
            .get("repairs_retries")
            .and_then(Value::as_u64)
            .is_some()
}

fn schema(object: &Map<String, Value>) -> bool {
    object.get("$schema").and_then(Value::as_str)
        == Some("https://json-schema.org/draft/2020-12/schema")
        && object.get("type").and_then(Value::as_str) == Some("object")
        && object.get("additionalProperties").and_then(Value::as_bool) == Some(false)
        && object
            .get("required")
            .and_then(Value::as_array)
            .is_some_and(|required| required.len() >= 4)
        && object
            .get("properties")
            .and_then(Value::as_object)
            .is_some_and(|properties| {
                properties.contains_key("schema") && properties.contains_key("results")
            })
}

fn components(object: &Map<String, Value>) -> bool {
    object.get("schema").and_then(Value::as_str)
        == Some("getcodexy.component-installation-cases.v1")
        && object
            .get("fixtures")
            .and_then(Value::as_array)
            .is_some_and(|fixtures| {
                !fixtures.is_empty()
                    && fixtures.iter().all(|fixture| {
                        fixture.as_object().is_some_and(|fixture| {
                            [
                                "id",
                                "command",
                                "requested_components",
                                "selection_before",
                                "selection_after",
                                "outcome",
                            ]
                            .iter()
                            .all(|key| fixture.contains_key(*key))
                        })
                    })
            })
}

fn nonempty(object: &Map<String, Value>, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}
