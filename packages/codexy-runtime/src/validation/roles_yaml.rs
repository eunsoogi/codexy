use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::paths::display_relative;
use crate::validation::prompt_yaml;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for skill_file in skill_files(&plugin_root.join("skills")) {
        let prompt = skill_file
            .parent()
            .unwrap_or(plugin_root)
            .join("agents/openai.yaml");
        if !prompt.exists() {
            errors.push(format!(
                "{} skill bundle is missing agents/openai.yaml",
                display_relative(skill_file.parent().unwrap_or(plugin_root))
            ));
        }
    }
    let top_level_prompt = plugin_root.join("agents/openai.yaml");
    if !top_level_prompt.exists() {
        errors.push(format!(
            "{} is required for plugin invocation metadata",
            display_relative(&top_level_prompt)
        ));
    }
    for path in openai_yaml_files(plugin_root) {
        errors.extend(
            check_yaml_file(plugin_root, &path).unwrap_or_else(|error| vec![error.to_string()]),
        );
    }
    errors
}

fn skill_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("SKILL.md"))
        .filter(|path| path.exists())
        .collect()
}

fn openai_yaml_files(plugin_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_openai_yaml(plugin_root, &mut files);
    files
}

fn collect_openai_yaml(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_openai_yaml(&path, files);
        } else if path.ends_with("openai.yaml")
            && path
                .parent()
                .and_then(Path::file_name)
                .and_then(|value| value.to_str())
                == Some("agents")
        {
            files.push(path);
        }
    }
}

fn check_yaml_file(plugin_root: &Path, path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let parsed = prompt_yaml::parse(&text, path)?;
    let mut errors = Vec::new();
    for field in ["display_name", "short_description", "default_prompt"] {
        if !matches!(prompt_yaml::get_path(&parsed, &["interface", field]), Some(prompt_yaml::Scalar::Text(text)) if !text.trim().is_empty())
        {
            errors.push(format!(
                "{} interface.{field} must be a non-empty string",
                display_relative(path)
            ));
        }
    }
    if path == plugin_root.join("agents/openai.yaml")
        && !matches!(
            prompt_yaml::get_path(&parsed, &["interface", "default_prompt"]),
            Some(prompt_yaml::Scalar::Text(text)) if text.contains("$codex-orchestration")
        )
    {
        errors.push(format!(
            "{} interface.default_prompt must route through $codex-orchestration",
            display_relative(path)
        ));
    }
    if !matches!(
        prompt_yaml::get_path(&parsed, &["policy", "allow_implicit_invocation"]),
        Some(prompt_yaml::Scalar::Bool(true))
    ) {
        errors.push(format!(
            "{} policy.allow_implicit_invocation must be true",
            display_relative(path)
        ));
    }
    Ok(errors)
}
