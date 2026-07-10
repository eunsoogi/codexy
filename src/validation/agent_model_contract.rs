use std::collections::BTreeSet;
use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

#[doc(hidden)]
#[derive(Debug)]
pub struct SpecialistModelContract {
    pub name: &'static str,
    pub model: &'static str,
    pub reasoning_effort: &'static str,
}

#[doc(hidden)]
pub const SPECIALIST_MODEL_CONTRACTS: &[SpecialistModelContract] = &[
    contract("codexy-architect", "gpt-5.6-sol", "high"),
    contract("codexy-auditor", "gpt-5.6-terra", "medium"),
    contract("codexy-cartographer", "gpt-5.6-luna", "low"),
    contract("codexy-forge", "gpt-5.6-terra", "medium"),
    contract("codexy-pathfinder", "gpt-5.6-sol", "xhigh"),
    contract("codexy-scribe", "gpt-5.6-luna", "low"),
    contract("codexy-sculptor", "gpt-5.6-terra", "high"),
    contract("codexy-sentinel", "gpt-5.6-sol", "xhigh"),
    contract("codexy-shipwright", "gpt-5.6-terra", "high"),
    contract("codexy-tracer", "gpt-5.6-sol", "high"),
    contract("codexy-warden", "gpt-5.6-sol", "xhigh"),
    contract("codexy-weaver", "gpt-5.6-terra", "medium"),
];

const fn contract(
    name: &'static str,
    model: &'static str,
    reasoning_effort: &'static str,
) -> SpecialistModelContract {
    SpecialistModelContract {
        name,
        model,
        reasoning_effort,
    }
}

pub(super) fn check_catalog_files(
    catalog_path: &Path,
    filenames: &[String],
    errors: &mut Vec<String>,
) {
    let actual = filenames.iter().cloned().collect::<BTreeSet<_>>();
    let expected = SPECIALIST_MODEL_CONTRACTS
        .iter()
        .map(SpecialistModelContract::filename)
        .collect::<BTreeSet<_>>();
    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();
    if !missing.is_empty() || !unexpected.is_empty() {
        errors.push(format!(
            "{} agent_files must exactly match the specialist model contract; missing: {}; unexpected: {}",
            display_relative(catalog_path),
            display_items(&missing),
            display_items(&unexpected)
        ));
    }
}

pub(super) fn check_agent(path: &Path, name: &str, agent: &Value, errors: &mut Vec<String>) {
    let Some(contract) = SPECIALIST_MODEL_CONTRACTS
        .iter()
        .find(|contract| contract.name == name)
    else {
        errors.push(format!(
            "{} {name} has no specialist model contract",
            display_relative(path)
        ));
        return;
    };
    check_field(path, name, agent, "model", contract.model, errors);
    check_field(
        path,
        name,
        agent,
        "model_reasoning_effort",
        contract.reasoning_effort,
        errors,
    );
}

impl SpecialistModelContract {
    fn filename(&self) -> String {
        format!("{}.toml", self.name)
    }
}

fn check_field(
    path: &Path,
    name: &str,
    agent: &Value,
    field: &str,
    expected: &str,
    errors: &mut Vec<String>,
) {
    if agent.get(field).and_then(Value::as_str) != Some(expected) {
        errors.push(format!(
            "{} {name} {field} must be {expected}",
            display_relative(path)
        ));
    }
}

fn display_items<T: AsRef<str>>(items: &[T]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
