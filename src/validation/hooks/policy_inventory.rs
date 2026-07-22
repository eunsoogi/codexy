use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;

use crate::paths::display_relative;
use crate::validation::load_json;

const INVENTORY_PATH: &str = "hooks/policy-inventory.json";
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
    version: u8,
    generated_from: String,
    test_suites: BTreeMap<String, String>,
    rules: Vec<Rule>,
    summary: Summary,
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
    if inventory.version != 5 || inventory.generated_from != "skills/**/*.md" {
        bail!(
            "{} must identify semantic Markdown and YAML inventory version 5",
            display_relative(&path)
        );
    }
    check_test_registry(&path, &inventory.test_suites)?;
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
        check_rule(&path, rule)?;
    }
    check_summary(&path, &inventory)
}

fn check_test_registry(path: &Path, registry: &BTreeMap<String, String>) -> Result<()> {
    for id in TEST_IDS {
        if !registry
            .get(*id)
            .is_some_and(|value| !value.trim().is_empty())
        {
            bail!("{} must register real test ID {id}", display_relative(path));
        }
    }
    Ok(())
}

fn check_rule(path: &Path, rule: &Rule) -> Result<()> {
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
    if rule.positive_tests.is_empty() || rule.negative_tests.is_empty() || rule.evidence.is_empty()
    {
        bail!(
            "{} rule {} must carry positive, negative, and evidence receipts",
            display_relative(path),
            rule.id
        );
    }
    match rule.decision.as_str() {
        "enforced" => {
            if rule.event != "PreToolUse"
                || rule.input == "unavailable"
                || rule.tests.is_empty()
                || rule.unavailable_event.is_some()
                || rule.unavailable_input.is_some()
                || rule.rationale.is_some()
            {
                bail!(
                    "{} rule {} overclaims preventive enforcement",
                    display_relative(path),
                    rule.id
                );
            }
        }
        "reviewed-exception" => {
            if rule.event != "unavailable"
                || rule.input != "unavailable"
                || !rule
                    .unavailable_event
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                || !rule
                    .unavailable_input
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                || !rule
                    .rationale
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                || !rule.evidence.iter().any(|item| item.contains("0.144.4"))
            {
                bail!(
                    "{} rule {} lacks an exact-build reviewed exception",
                    display_relative(path),
                    rule.id
                );
            }
        }
        _ => bail!(
            "{} rule {} remains uncovered",
            display_relative(path),
            rule.id
        ),
    }
    Ok(())
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
