use serde_yaml::{Mapping, Value};

pub(crate) fn mapping_field<'a>(
    mapping: &'a Mapping,
    key: &str,
    context: &str,
) -> Result<&'a Mapping, Box<dyn std::error::Error>> {
    mapping
        .get(key)
        .and_then(Value::as_mapping)
        .ok_or_else(|| format!("{context} missing mapping {key}").into())
}

pub(crate) fn string_field<'a>(
    mapping: &'a Mapping,
    key: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    mapping
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field {key}").into())
}

pub(crate) fn string_sequence(value: &Value) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    value
        .as_sequence()
        .ok_or("expected sequence")?
        .iter()
        .map(|item| item.as_str().ok_or_else(|| "expected string item".into()))
        .collect()
}

pub(crate) fn steps(job: &Mapping) -> Result<&[Value], Box<dyn std::error::Error>> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .ok_or_else(|| "job missing steps".into())
}

pub(crate) fn step<'a>(
    steps: &'a [Value],
    name: &str,
) -> Result<&'a Mapping, Box<dyn std::error::Error>> {
    steps
        .iter()
        .filter_map(Value::as_mapping)
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("missing workflow step {name}").into())
}
