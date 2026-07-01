use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::paths::display_relative;
use crate::validation::{instruction_policy_text, load_json, prompt_yaml};

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let Ok(surfaces) = instruction_surfaces(plugin_root) else {
        return vec![format!(
            "{} agent instruction surfaces could not be read",
            display_relative(plugin_root)
        )];
    };
    check_surfaces(surfaces, &mut errors);
    errors
}

pub(super) fn check_roles(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let Ok(surfaces) = role_instruction_surfaces(plugin_root) else {
        return vec![format!(
            "{} role instruction surfaces could not be read",
            display_relative(&plugin_root.join("agents"))
        )];
    };
    check_surfaces(surfaces, &mut errors);
    errors
}

fn check_surfaces(surfaces: Vec<PathBuf>, errors: &mut Vec<String>) {
    for path in surfaces {
        match fs::read_to_string(&path) {
            Ok(text) => {
                if !path.ends_with(".codex-plugin/plugin.json") {
                    instruction_policy_text::check_text(&path, &text, errors, false);
                }
                check_structured_prompts(&path, &text, errors);
            }
            Err(error) => errors.push(format!(
                "{} could not be read: {error}",
                display_relative(&path)
            )),
        }
    }
}

fn check_structured_prompts(path: &Path, text: &str, errors: &mut Vec<String>) {
    if path.ends_with(".codex-plugin/plugin.json") {
        if let Ok(manifest) = load_json(path) {
            if let Some(items) = manifest
                .get("interface")
                .and_then(|value| value.get("defaultPrompt"))
                .and_then(serde_json::Value::as_array)
            {
                for item in items {
                    if let Some(prompt) = item.as_str() {
                        instruction_policy_text::check_text(path, prompt, errors, true);
                    }
                }
            }
        }
    } else if is_openai_yaml(path) {
        if let Ok(parsed) = prompt_yaml::parse(text, path) {
            if is_plugin_agent_openai_yaml(path) {
                check_yaml_prompt_scalars(path, &parsed, errors);
            } else if let Some(prompt_yaml::Scalar::Text(prompt)) =
                prompt_yaml::get_path(&parsed, &["interface", "default_prompt"])
            {
                instruction_policy_text::check_text(path, prompt, errors, true);
                check_yaml_prompt_path(path, &parsed, &["guidance"], errors);
            }
        }
    }
}

fn check_yaml_prompt_path(
    path: &Path,
    items: &BTreeMap<String, prompt_yaml::Scalar>,
    keys: &[&str],
    errors: &mut Vec<String>,
) {
    if let Some(prompt_yaml::Scalar::Text(prompt)) = prompt_yaml::get_path(items, keys) {
        instruction_policy_text::check_text(path, prompt, errors, true);
    }
}

fn check_yaml_prompt_scalars(
    path: &Path,
    items: &BTreeMap<String, prompt_yaml::Scalar>,
    errors: &mut Vec<String>,
) {
    for (key, item) in items {
        if matches!(key.as_str(), "display_name" | "short_description") {
            continue;
        }
        match item {
            prompt_yaml::Scalar::Text(prompt) => {
                instruction_policy_text::check_text(path, prompt, errors, true);
            }
            prompt_yaml::Scalar::Map(children) => check_yaml_prompt_scalars(path, children, errors),
            prompt_yaml::Scalar::Bool(_) => {}
        }
    }
}

fn is_openai_yaml(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("openai.yaml")
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("agents")
}

fn is_plugin_agent_openai_yaml(path: &Path) -> bool {
    is_openai_yaml(path)
        && path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("codexy")
}

fn instruction_surfaces(plugin_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    if let Some(repo_root) = plugin_root.parent().and_then(Path::parent) {
        let path = repo_root.join("AGENTS.md");
        if path.exists() {
            paths.insert(path);
        }
    }
    paths.insert(plugin_root.join(".codex-plugin/plugin.json"));
    paths.insert(plugin_root.join("agents/openai.yaml"));
    for entry in fs::read_dir(plugin_root.join("agents"))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml")
            && path.file_name().and_then(|name| name.to_str()) != Some("catalog.toml")
        {
            paths.insert(path);
        }
    }
    collect_skill_surfaces(&plugin_root.join("skills"), &mut paths)?;
    Ok(paths.into_iter().collect())
}

fn role_instruction_surfaces(plugin_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    paths.insert(plugin_root.join("agents/openai.yaml"));
    collect_agent_toml_surfaces(plugin_root, &mut paths)?;
    collect_agent_prompt_surfaces(plugin_root, &mut paths)?;
    Ok(paths.into_iter().collect())
}

fn collect_agent_toml_surfaces(
    plugin_root: &Path,
    paths: &mut BTreeSet<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(plugin_root.join("agents"))? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml")
            && path.file_name().and_then(|name| name.to_str()) != Some("catalog.toml")
        {
            paths.insert(path);
        }
    }
    Ok(())
}

fn collect_agent_prompt_surfaces(
    plugin_root: &Path,
    paths: &mut BTreeSet<PathBuf>,
) -> std::io::Result<()> {
    collect_agent_prompt_surfaces_from(&plugin_root.join("skills"), paths)
}

fn collect_agent_prompt_surfaces_from(
    root: &Path,
    paths: &mut BTreeSet<PathBuf>,
) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_agent_prompt_surfaces_from(&path, paths)?;
        } else if path.ends_with("agents/openai.yaml") {
            paths.insert(path);
        }
    }
    Ok(())
}

fn collect_skill_surfaces(root: &Path, paths: &mut BTreeSet<PathBuf>) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_skill_surfaces(&path, paths)?;
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
            || path.extension().and_then(|ext| ext.to_str()) == Some("md")
            || path.ends_with("agents/openai.yaml")
        {
            paths.insert(path);
        }
    }
    Ok(())
}
