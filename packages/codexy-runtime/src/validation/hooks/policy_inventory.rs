use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::paths::display_relative;
use crate::validation::load_json;

use super::policy_inventory_contract::{self, CapabilityContract};
use super::policy_inventory_suite::{self, RUNTIME_SUITE};

const INVENTORY_PATH: &str = "hooks/policy-inventory.json";
const INVENTORY_SCHEMA: &str = "codexy.hooks.policy-inventory";
const TEST_IDS: &[&str] = &[
    "admission",
    "inventory",
    "postcompact",
    "thread-routing",
    "topology",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    schema: String,
    generated_from: String,
    capability_contract: ContractBinding,
    test_suites: BTreeMap<String, String>,
    rules: Vec<Rule>,
    summary: Summary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContractBinding {
    schema: String,
    content_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Rule {
    id: String,
    digest: String,
    source: String,
    text: String,
    event: String,
    input: String,
    decision: String,
    tests: Vec<String>,
    unavailable_event: Option<String>,
    unavailable_input: Option<String>,
    evidence: Vec<String>,
    rationale: Option<String>,
    positive_tests: Vec<String>,
    negative_tests: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Summary {
    total: usize,
    enforced: usize,
    reviewed_exceptions: usize,
    uncovered: usize,
}

pub(super) fn check(plugin_root: &Path) -> Result<()> {
    let path = plugin_root.join(INVENTORY_PATH);
    let inventory: Inventory = serde_json::from_value(load_json(&path)?).map_err(|error| {
        anyhow!(
            "{} must match policy inventory schema: {error}",
            display_relative(&path)
        )
    })?;
    if inventory.schema != INVENTORY_SCHEMA || inventory.generated_from != "skills/**/*.md" {
        bail!(
            "{} must identify the stable inventory schema and semantic Markdown input",
            display_relative(&path)
        );
    }
    let contract = policy_inventory_contract::check(plugin_root)?;
    policy_inventory_contract::check_binding(
        &inventory.capability_contract.schema,
        &inventory.capability_contract.content_digest,
        &contract,
    )?;
    check_test_registry(plugin_root, &path, &inventory.test_suites)?;
    let discovered = super::policy_inventory_discovery::discover(plugin_root)?;
    if inventory.rules.len() != discovered.len() {
        bail!(
            "{} has uncovered normative rules: inventory={}, discovered={}",
            display_relative(&path),
            inventory.rules.len(),
            discovered.len()
        );
    }
    let mut ids = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for (rule, found) in inventory.rules.iter().zip(&discovered) {
        if rule.id != found.id
            || rule.digest != found.digest
            || rule.source != found.source
            || rule.text != found.text
        {
            bail!(
                "{} has an unreviewed, moved, or changed normative rule at {}",
                display_relative(&path),
                found.source
            );
        }
        if !ids.insert(&rule.id) || !sources.insert(&rule.source) {
            bail!(
                "{} rule IDs and sources must be unique",
                display_relative(&path)
            );
        }
        check_rule(&path, rule, &contract)?;
    }
    check_summary(&path, &inventory)
}

fn check_test_registry(
    plugin_root: &Path,
    path: &Path,
    registry: &BTreeMap<String, String>,
) -> Result<()> {
    if registry.len() != TEST_IDS.len()
        || TEST_IDS
            .iter()
            .any(|id| registry.get(*id) != Some(&RUNTIME_SUITE.to_owned()))
    {
        bail!(
            "{} must map each policy test ID to the actual admission runtime suite",
            display_relative(path)
        );
    }
    let suite = policy_inventory_suite::runtime_path(plugin_root)?;
    let metadata = std::fs::symlink_metadata(&suite).map_err(|error| {
        anyhow!(
            "{} must resolve the actual admission runtime suite as a regular file: {error}",
            display_relative(&suite)
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "{} must resolve the actual admission runtime suite as a regular file",
            display_relative(&suite)
        );
    }
    Ok(())
}

fn check_rule(path: &Path, rule: &Rule, contract: &CapabilityContract) -> Result<()> {
    for id in rule
        .tests
        .iter()
        .chain(&rule.positive_tests)
        .chain(&rule.negative_tests)
    {
        if !TEST_IDS.contains(&id.as_str()) {
            bail!(
                "{} rule {} references unknown test ID {id}",
                display_relative(path),
                rule.id
            );
        }
    }
    if rule.positive_tests.is_empty()
        || rule.negative_tests.is_empty()
        || !rule.evidence.contains(&contract.evidence())
    {
        bail!(
            "{} rule {} must carry capability-bound positive, negative, and evidence receipts",
            display_relative(path),
            rule.id
        );
    }
    match rule.decision.as_str() {
        "enforced"
            if contract.prevents(&rule.event, &rule.input)
                && !rule.tests.is_empty()
                && rule.unavailable_event.is_none()
                && rule.unavailable_input.is_none()
                && rule.rationale.is_none() =>
        {
            Ok(())
        }
        "reviewed-exception"
            if rule.event == "unavailable"
                && rule.input == "unavailable"
                && non_empty(&rule.unavailable_event)
                && non_empty(&rule.unavailable_input)
                && non_empty(&rule.rationale) =>
        {
            Ok(())
        }
        "enforced" => bail!(
            "{} rule {} overclaims preventive enforcement",
            display_relative(path),
            rule.id
        ),
        "reviewed-exception" => bail!(
            "{} rule {} lacks an audited nonpreventive exception",
            display_relative(path),
            rule.id
        ),
        _ => bail!(
            "{} rule {} remains uncovered",
            display_relative(path),
            rule.id
        ),
    }
}

fn non_empty(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|item| !item.trim().is_empty())
}

fn check_summary(path: &Path, inventory: &Inventory) -> Result<()> {
    let enforced = inventory
        .rules
        .iter()
        .filter(|rule| rule.decision == "enforced")
        .count();
    let reviewed = inventory
        .rules
        .iter()
        .filter(|rule| rule.decision == "reviewed-exception")
        .count();
    if inventory.summary.total != inventory.rules.len()
        || inventory.summary.enforced != enforced
        || inventory.summary.reviewed_exceptions != reviewed
        || inventory.summary.uncovered != 0
        || enforced + reviewed != inventory.rules.len()
    {
        bail!(
            "{} summary must prove uncovered=0 from explicit decisions",
            display_relative(path)
        );
    }
    Ok(())
}
