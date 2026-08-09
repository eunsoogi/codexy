type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::{collections::BTreeMap, path::Path};

const WORKFLOWS: &[(&str, &str)] = &[
    ("init", "Keep"),
    ("ingest", "Keep"),
    ("ingest-collection", "Merge"),
    ("collect", "Remove"),
    ("compile", "Keep"),
    ("query", "Keep"),
    ("refresh", "Keep"),
    ("lint", "Keep"),
    ("librarian", "Merge"),
    ("audit", "Merge"),
    ("research", "Merge"),
    ("output", "Merge"),
    ("plan", "Remove"),
    ("project", "Remove"),
    ("inventory", "Remove"),
    ("dataset", "Remove"),
    ("archive", "Remove"),
    ("ll", "Remove"),
    ("assess", "Merge"),
    ("retract", "Merge"),
    ("thesis", "Merge"),
    ("status", "Remove"),
    ("session", "Remove"),
    ("session-capture", "Remove"),
    ("rehydrate", "Remove"),
    ("session-promote", "Merge"),
    ("feedback", "Remove"),
    ("feedback-capture", "Remove"),
    ("feedback-promote", "Merge"),
];
const LIMITS: &[(&str, usize)] = &[
    ("query.max_index_files", 3),
    ("query.max_article_files", 8),
    ("query.max_index_file_bytes", 4_000),
    ("query.max_article_file_bytes", 4_000),
    ("query.max_total_bytes", 48_000),
    ("freshness.threshold", 70),
    ("freshness.hot_half_life_days", 7),
    ("freshness.warm_half_life_days", 30),
    ("freshness.cold_half_life_days", 180),
];
const FORMULAS: &[&str] = &[
    "freshness.decay = 25 * 0.5^(age_days / half_life_days)",
    "freshness.source_chain = 25 * resolvable_sources / total_sources",
    "freshness.conversation = min(100, 2 * (verification + compilation))",
];

#[test]
fn wiki_skill_exposes_a_complete_measurable_minimal_contract() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;
    let contract = std::fs::read_to_string(
        root.join("plugins/codexy/skills/wiki/references/minimal-contract.md"),
    )?;

    assert!(skill.contains("[Minimal Contract](references/minimal-contract.md)"));
    validate_contract(&contract)?;
    Ok(())
}

#[test]
fn contract_parser_rejects_incomplete_or_non_measurable_contracts() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract = std::fs::read_to_string(
        root.join("plugins/codexy/skills/wiki/references/minimal-contract.md"),
    )?;
    for mutation in [
        contract.replacen("| `retract` | Merge |", "", 1),
        contract.replacen("| `compile` | Keep |", "| `compile` | Keep |\n| `compile` | Keep |", 1),
        contract.replacen("| `init` | Keep |", "| `init` | Merge |", 1),
        contract.replacen("## Current workflow disposition", "### Current workflow disposition", 1),
        contract.replacen("### Ingest", "#### Ingest", 1),
        contract.replacen("query.max_index_files = 3", "query.max_index_files = three", 1),
        contract.replacen(FORMULAS[0], "freshness.decay = unspecified", 1),
        contract.replacen("query.max_index_files = 3", "query.max_index_files = 4", 1),
    ] {
        assert!(validate_contract(&mutation).is_err());
    }
    Ok(())
}

struct Contract {
    workflows: BTreeMap<String, String>,
    limits: BTreeMap<String, usize>,
    measures: String,
}

fn parse_contract(text: &str) -> Result<Contract, String> {
    let workflows = section(text, "## Current workflow disposition")?;
    let measures = section(text, "### Machine-checkable limits")?;
    let essential = section(text, "## Essential contract")?;
    let measurable = section(text, "## Measurable criteria")?;
    for heading in [
        "### Ingest",
        "### Compile",
        "### Query",
        "### Refresh and provenance",
        "### Bounded context",
    ] {
        section(&essential, heading)?;
    }
    for heading in ["### Context efficiency", "### Traceability", "### Freshness"] {
        section(&measurable, heading)?;
    }
    Ok(Contract {
        workflows: workflow_rows(&workflows)?,
        limits: numeric_assignments(&measures),
        measures,
    })
}

fn validate_contract(text: &str) -> Result<(), String> {
    let parsed = parse_contract(text)?;
    if parsed.workflows.len() != WORKFLOWS.len() {
        return Err("incomplete workflow inventory".into());
    }
    for (workflow, disposition) in WORKFLOWS {
        if parsed.workflows.get(*workflow) != Some(&disposition.to_string()) {
            return Err(format!("invalid disposition for {workflow}"));
        }
    }
    for (key, value) in LIMITS {
        if parsed.limits.get(*key) != Some(value) {
            return Err(format!("invalid limit {key}"));
        }
    }
    for formula in FORMULAS {
        if !parsed.measures.contains(formula) {
            return Err(format!("missing formula {formula}"));
        }
    }
    Ok(())
}

fn section(text: &str, title: &str) -> Result<String, String> {
    let level = title.chars().take_while(|character| *character == '#').count();
    let mut found = false;
    let mut fenced = false;
    let mut body = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if !fenced && line == title {
            if found {
                return Err(format!("duplicate section {title}"));
            }
            found = true;
            continue;
        }
        let next_level = line.chars().take_while(|character| *character == '#').count();
        if found && !fenced && next_level > 0 && next_level <= level && line.as_bytes().get(next_level) == Some(&b' ') {
            break;
        }
        if found {
            body.push_str(line);
            body.push('\n');
        }
    }
    found.then_some(body).ok_or_else(|| format!("missing section {title}"))
}

fn workflow_rows(table: &str) -> Result<BTreeMap<String, String>, String> {
    let mut rows = table.lines().filter(|line| line.trim_start().starts_with('|'));
    let header = rows.next().ok_or("missing workflow table")?;
    if cells(header)? != ["Current workflow", "Disposition", "Contract role"] {
        return Err("invalid workflow table header".into());
    }
    rows.next().ok_or("missing workflow table separator")?;
    let mut workflows = BTreeMap::new();
    for row in rows {
        let cells = cells(row)?;
        if cells.len() != 3 || cells[2].is_empty() {
            return Err("malformed workflow row".into());
        }
        let name = cells[0].strip_prefix('`').and_then(|name| name.strip_suffix('`'))
            .ok_or("workflow name must be code-formatted")?;
        if !matches!(cells[1], "Keep" | "Merge" | "Remove") || workflows.insert(name.into(), cells[1].into()).is_some() {
            return Err("duplicate or invalid workflow disposition".into());
        }
    }
    Ok(workflows)
}

fn cells(row: &str) -> Result<Vec<&str>, String> {
    let row = row.trim();
    if !row.starts_with('|') || !row.ends_with('|') {
        return Err("malformed table row".into());
    }
    Ok(row[1..row.len() - 1].split('|').map(str::trim).collect())
}

fn numeric_assignments(section: &str) -> BTreeMap<String, usize> {
    section
        .lines()
        .filter_map(|line| line.trim().trim_matches('`').split_once(" = "))
        .filter_map(|(key, value)| value.parse().ok().map(|value| (key.into(), value)))
        .collect()
}

#[test]
fn minimal_contract_uses_canonical_instruction_policy_forms() -> TestResult {
    let fixture = crate::support::instruction_policy_fixture(Path::new(
        "skills/wiki/references/minimal-contract.md",
    ))?;
    let contract_path = fixture.path();
    let contract = std::fs::read_to_string(&contract_path)?;

    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));

    let invalid_prohibition = contract.replace(
        "MUST NOT overwrite raw history.",
        "never overwrites raw history.",
    );
    std::fs::write(&contract_path, invalid_prohibition)?;
    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("prohibitions must use MUST NOT"));

    let invalid_imperative = contract.replace(
        "MUST report why it remains current.",
        "report why it remains current.",
    );
    std::fs::write(&contract_path, invalid_imperative)?;
    let output = crate::support::validator_instruction_policy_file(&contract_path)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("mandatory instructions must use MUST"));
    Ok(())
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
