use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::support::TestResult;

#[path = "architecture_docs_inventory/skill_inventory.rs"]
mod skill_inventory;
use skill_inventory::{assert_local_links, packaged_skills, skill_path_consumer_count};

#[path = "architecture_docs_inventory/cargo_targets.rs"]
mod cargo_targets;
#[path = "architecture_docs_inventory/mcp_inventory.rs"]
mod mcp_inventory;
#[path = "architecture_docs_inventory/module_roots.rs"]
mod module_roots;
#[path = "architecture_docs_inventory/readme_skills.rs"]
mod readme_skills;
#[path = "architecture_docs_inventory/repository_skills.rs"]
mod repository_skills;

#[derive(Debug, Eq, PartialEq)]
struct Agent {
    model: String,
    effort: String,
}

#[test]
fn architecture_guide_matches_packaged_inventory() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = skill_inventory::combined(root)?;
    validate_guide(root, &guide).map_err(Into::into)
        .and_then(|()| {
            let documented = skill_path_consumer_count(&guide)?;
            let discovered = packaged_skills(root)?.len();
            (documented == discovered)
                .then_some(())
                .ok_or_else(|| format!("skill path-consumer count {documented} differs from discovered inventory {discovered}"))
        })
        .map_err(Into::into)
}

#[test]
fn architecture_inventory_rejects_omissions_duplicates_and_stale_fields() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = skill_inventory::combined(root)?;
    let agent_row = first_row(&guide, "Specialist agents")?;
    let skill_row = first_row(&guide, "Packaged skills")?;
    let mcp_row = first_row(&guide, "MCP servers")?;

    assert!(validate_guide(root, &guide.replacen(&agent_row, "", 1)).is_err());
    assert!(validate_guide(root, &guide.replacen(&skill_row, "", 1)).is_err());
    assert!(validate_guide(root, &guide.replacen(&mcp_row, "", 1)).is_err());
    assert!(
        validate_guide(
            root,
            &guide.replacen(&agent_row, &format!("{agent_row}\n{agent_row}"), 1)
        )
        .is_err()
    );
    assert!(validate_guide(root, &guide.replacen("`gpt-6-sol`", "`stale-model`", 1)).is_err());
    assert!(validate_guide(root, &guide.replacen("`xhigh`", "`stale-effort`", 1)).is_err());
    assert!(
        validate_guide(
            root,
            &guide.replacen("./mcp/codexy_mcp_bootstrap.py", "./mcp/stale-devtools", 1)
        )
        .is_err()
    );
    assert!(validate_guide(root, &guide.replacen("--stdio", "--stale-stdio", 1)).is_err());
    assert!(
        validate_guide(
            root,
            &guide.replacen("\"cwd\":\".\"", "\"cwd\":\"stale-cwd\"", 1)
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn readmes_link_to_the_public_guide_and_mermaid_workflows_are_present() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let guide = skill_inventory::combined(root)?;
    let english = std::fs::read_to_string(root.join("README.md"))?;
    let korean = std::fs::read_to_string(root.join("README.ko.md"))?;

    assert_eq!(
        readme_skills::link_count(&english, "docs/architecture.md"),
        1
    );
    assert_eq!(
        readme_skills::link_count(&korean, "docs/architecture.md"),
        1
    );
    assert_eq!(guide.matches("```mermaid").count(), 2);
    assert!(
        readme_skills::has_word(&guide, "configured")
            && readme_skills::has_word(&guide, "callable")
    );
    for path in skill_inventory::DOCUMENTS {
        let document = std::fs::read_to_string(root.join(path))?;
        assert_local_links(root, &root.join(path), &document)?;
    }
    assert_local_links(root, &root.join("README.md"), &english)?;
    assert_local_links(root, &root.join("README.ko.md"), &korean)?;
    Ok(())
}

fn validate_guide(root: &Path, guide: &str) -> Result<(), String> {
    let documented_agents = agent_rows(guide)?;
    let expected_agents = packaged_agents(root)?;
    if documented_agents != expected_agents {
        return Err(format!("agent inventory differs: {documented_agents:?}"));
    }

    exact_names(
        &rows(guide, "Packaged skills")?,
        &packaged_skills(root)?,
        "skill",
        4,
    )?;
    let documented_mcps = mcp_inventory::documented(guide)?;
    let expected_mcps = mcp_inventory::packaged(root)?;
    if documented_mcps != expected_mcps {
        return Err(format!("MCP registrations differ: {documented_mcps:?}"));
    }
    Ok(())
}

fn packaged_agents(root: &Path) -> Result<BTreeMap<String, Agent>, String> {
    let agents_root = root.join("plugins/codexy/agents");
    let catalog = parse_toml(&agents_root.join("catalog.toml"))?;
    let files = catalog
        .get("agent_files")
        .and_then(toml::Value::as_array)
        .ok_or("catalog agent_files missing")?;
    let mut agents = BTreeMap::new();
    for file in files {
        let filename = file.as_str().ok_or("catalog filename must be text")?;
        let value = parse_toml(&agents_root.join(filename))?;
        let name = text_field(&value, "name")?;
        let agent = Agent {
            model: text_field(&value, "model")?,
            effort: text_field(&value, "model_reasoning_effort")?,
        };
        if agents.insert(name, agent).is_some() {
            return Err(format!("duplicate packaged agent in catalog: {filename}"));
        }
    }
    Ok(agents)
}

fn agent_rows(guide: &str) -> Result<BTreeMap<String, Agent>, String> {
    let mut agents = BTreeMap::new();
    for row in rows(guide, "Specialist agents")? {
        if row.len() != 4 {
            return Err(format!("agent row must have four columns: {row:?}"));
        }
        let agent = Agent {
            model: row[1].clone(),
            effort: row[2].clone(),
        };
        if agents.insert(row[0].clone(), agent).is_some() {
            return Err(format!("duplicate documented agent: {}", row[0]));
        }
    }
    Ok(agents)
}

fn exact_names(
    rows: &[Vec<String>],
    expected: &BTreeSet<String>,
    kind: &str,
    columns: usize,
) -> Result<(), String> {
    if rows
        .iter()
        .any(|row| row.len() != columns || row.iter().any(String::is_empty))
    {
        return Err(format!("{kind} rows must have {columns} non-empty columns"));
    }
    let names = rows.iter().map(|row| row[0].clone()).collect::<Vec<_>>();
    let unique = names.iter().cloned().collect::<BTreeSet<_>>();
    if names.len() != unique.len() || &unique != expected {
        return Err(format!("{kind} inventory differs: {names:?}"));
    }
    Ok(())
}

fn rows(guide: &str, heading: &str) -> Result<Vec<Vec<String>>, String> {
    let section = section(guide, heading)?;
    let rows = section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return Err(format!("{heading} table has no inventory rows"));
    }
    Ok(rows)
}

fn section<'a>(guide: &'a str, heading: &str) -> Result<&'a str, String> {
    let marker = format!("## {heading}");
    let start = guide
        .find(&marker)
        .ok_or_else(|| format!("missing heading: {marker}"))?;
    let remainder = &guide[start + marker.len()..];
    Ok(remainder.split("\n## ").next().unwrap_or(remainder))
}

fn first_row(guide: &str, heading: &str) -> Result<String, String> {
    section(guide, heading)?
        .lines()
        .find(|line| line.starts_with("| `"))
        .map(str::to_owned)
        .ok_or_else(|| format!("missing row in {heading}"))
}

fn parse_toml(path: &PathBuf) -> Result<toml::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    toml::from_str(&text).map_err(|error| error.to_string())
}

fn text_field(value: &toml::Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing text field: {field}"))
}
