use serde_json::Value;

const EXPECTED_SCHEMA: &str = include_str!(
    "../../../../plugins/codexy/skills/orchestration/references/routing-evaluation-results.schema.json"
);

pub(super) fn is_closed(schema: &Value) -> bool {
    serde_json::from_str::<Value>(EXPECTED_SCHEMA).is_ok_and(|expected| schema == &expected)
}
