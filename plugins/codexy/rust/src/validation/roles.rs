use std::{collections::BTreeSet, fs, path::Path};

use anyhow::{Context as _, Result};
use toml::Value;

use crate::paths::display_relative;
use crate::validation::{load_toml, roles_yaml, toml_array_strings};

const REQUIRED_AGENTS: &[&str] = &[
    "planner",
    "explorer",
    "architect",
    "implementer",
    "debugger",
    "qa",
    "refactor",
    "reviewer",
    "integrator",
    "release",
    "security",
    "documenter",
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    errors.extend(check_specialists(plugin_root).unwrap_or_else(|error| vec![error.to_string()]));
    errors.extend(check_project_agents(plugin_root));
    errors.extend(check_agent_yaml(plugin_root));
    errors
}

fn check_specialists(plugin_root: &Path) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    let agents_root = plugin_root.join("agents");
    let catalog_path = agents_root.join("catalog.toml");
    if agents_root.join("roles").exists() {
        errors.push(format!(
            "{} must not contain specialist agent definitions; store each specialist agent in agents/<name>.toml",
            display_relative(&agents_root.join("roles"))
        ));
    }
    if agents_root.join("roles.toml").exists() {
        errors.push(format!(
            "{} must not contain collapsed multi-role metadata; store each specialist agent in agents/<name>.toml",
            display_relative(&agents_root.join("roles.toml"))
        ));
    }
    let catalog = load_toml(&catalog_path)?;
    if catalog.get("default_branch_prefix").and_then(Value::as_str) == Some("eunsoogi/") {
        errors.push(format!(
            "{} default_branch_prefix must not be 'eunsoogi/'",
            display_relative(&catalog_path)
        ));
    }
    let agent_names = toml_array_strings(catalog.get("agent_files")).unwrap_or_default();
    if agent_names.is_empty() {
        errors.push(format!(
            "{} agent_files must be a list of agent TOML filenames",
            display_relative(&catalog_path)
        ));
        return Ok(errors);
    }
    let mut seen_files = BTreeSet::new();
    let mut seen_agents = BTreeSet::new();
    for filename in &agent_names {
        if !seen_files.insert(filename.clone()) {
            errors.push(format!(
                "{} agent_files must not contain duplicates",
                display_relative(&catalog_path)
            ));
        }
        if filename.contains('/')
            || filename.starts_with('.')
            || !Path::new(filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
        {
            errors.push(format!(
                "{} invalid agent file entry: {filename:?}",
                display_relative(&catalog_path)
            ));
            continue;
        }
        let path = agents_root.join(filename);
        if !path.exists() {
            errors.push(format!(
                "{} references missing agent file: {filename}",
                display_relative(&catalog_path)
            ));
            continue;
        }
        check_agent_file(&path, &mut seen_agents, &mut errors);
    }
    for entry in fs::read_dir(&agents_root)
        .with_context(|| format!("reading {}", display_relative(&agents_root)))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml")
            && path.file_name().and_then(|value| value.to_str()) != Some("catalog.toml")
            && !agent_names.iter().any(|name| path.ends_with(name))
        {
            errors.push(format!(
                "{} missing agent_files entry: {}",
                display_relative(&catalog_path),
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("unknown")
            ));
        }
    }
    let missing = REQUIRED_AGENTS
        .iter()
        .filter(|agent| !seen_agents.contains(**agent))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        errors.push(format!(
            "{} missing specialist agents: {}",
            display_relative(&agents_root),
            missing.join(", ")
        ));
    }
    Ok(errors)
}

fn check_agent_file(path: &Path, seen: &mut BTreeSet<String>, errors: &mut Vec<String>) {
    let Ok(agent) = load_toml(path) else {
        errors.push(format!("invalid TOML in {}", display_relative(path)));
        return;
    };
    if agent.get("roles").is_some() {
        errors.push(format!(
            "{} must define exactly one specialist agent and must not contain [[roles]]",
            display_relative(path)
        ));
    }
    let name = agent.get("name").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() {
        errors.push(format!(
            "{} name must be a non-empty string",
            display_relative(path)
        ));
        return;
    }
    if path.file_stem().and_then(|value| value.to_str()) != Some(name) {
        errors.push(format!(
            "{} filename must match agent name {name:?}",
            display_relative(path)
        ));
    }
    if !seen.insert(name.to_owned()) {
        errors.push(format!(
            "{} duplicate agent name: {name}",
            display_relative(path)
        ));
    }
    if name == "orchestrator" {
        errors.push(format!(
            "{} assignable child orchestrator agent is not allowed",
            display_relative(path)
        ));
    }
    for field in ["display_name", "model", "effort", "when_to_use"] {
        if agent
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            errors.push(format!(
                "{} {field} must be a non-empty string",
                display_relative(path)
            ));
        }
    }
    for field in ["inputs", "outputs", "constraints"] {
        if toml_array_strings(agent.get(field)).is_none_or(|items| items.is_empty()) {
            errors.push(format!(
                "{} {field} must be a list of non-empty strings",
                display_relative(path)
            ));
        }
    }
}

fn check_project_agents(plugin_root: &Path) -> Vec<String> {
    let agents_dir = plugin_root.join(".codex/agents");
    if agents_dir.exists() {
        vec![format!(
            "{} is not loaded from an installed plugin; keep plugin-packaged specialist agent definitions in agents/<name>.toml",
            display_relative(&agents_dir)
        )]
    } else {
        Vec::new()
    }
}

fn check_agent_yaml(plugin_root: &Path) -> Vec<String> {
    roles_yaml::check(plugin_root)
}
