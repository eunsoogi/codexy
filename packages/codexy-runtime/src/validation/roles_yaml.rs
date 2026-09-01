use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_yaml::{Mapping, Value};

use crate::paths::display_relative;
use crate::validation::prompt_yaml;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let mut roots = vec![plugin_root.join("skills")];
    if let Some(repo_root) = super::repository_skill_root::from_plugin_root(plugin_root) {
        roots.push(repo_root.join(".agents/skills"));
    }
    for skill_file in roots.iter().flat_map(|root| skill_files(root)) {
        if !skill_file.is_file() {
            errors.push(format!(
                "{} skill bundle is missing SKILL.md",
                display_relative(skill_file.parent().unwrap_or(plugin_root))
            ));
            continue;
        }
        errors.extend(check_skill_frontmatter(&skill_file));
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
    for path in openai_yaml_files(plugin_root, &roots) {
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
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path().join("SKILL.md"))
        .collect()
}

fn check_skill_frontmatter(skill_file: &Path) -> Vec<String> {
    let text = match fs::read_to_string(skill_file) {
        Ok(text) => text,
        Err(error) => return vec![format!("{}: {error}", display_relative(skill_file))],
    };
    let frontmatter = match frontmatter(&text, skill_file) {
        Ok(frontmatter) => frontmatter,
        Err(error) => return vec![error.to_string()],
    };
    let parsed = match serde_yaml::from_str::<Mapping>(frontmatter) {
        Ok(parsed) => parsed,
        Err(error) => {
            return vec![format!(
                "{} frontmatter must be valid YAML: {error}",
                display_relative(skill_file)
            )];
        }
    };
    let mut errors = Vec::new();
    let name = match yaml_string(&parsed, "name") {
        Some(name) if !name.trim().is_empty() => name,
        _ => {
            errors.push(format!(
                "{} frontmatter.name must be a non-empty string",
                display_relative(skill_file)
            ));
            return errors;
        }
    };
    let expected = skill_file
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name != expected {
        errors.push(format!(
            "{} frontmatter name must match skill directory `{expected}`",
            display_relative(skill_file)
        ));
    }
    if yaml_string(&parsed, "description").is_none_or(|description| description.trim().is_empty()) {
        errors.push(format!(
            "{} frontmatter.description must be a non-empty string",
            display_relative(skill_file)
        ));
    }
    errors
}

fn yaml_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    mapping
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_str)
}

fn frontmatter<'a>(text: &'a str, skill_file: &Path) -> Result<&'a str> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (remainder, delimiter) = if let Some(remainder) = text.strip_prefix("---\n") {
        (remainder, "\n---\n")
    } else if let Some(remainder) = text.strip_prefix("---\r\n") {
        (remainder, "\r\n---\r\n")
    } else {
        anyhow::bail!(
            "{} frontmatter must open with ---",
            display_relative(skill_file)
        );
    };
    remainder
        .split_once(delimiter)
        .map(|(frontmatter, _)| frontmatter)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} frontmatter must close with ---",
                display_relative(skill_file)
            )
        })
}

fn openai_yaml_files(plugin_root: &Path, skill_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_openai_yaml(plugin_root, &mut files);
    for root in skill_roots {
        collect_openai_yaml(root, &mut files);
    }
    files.sort();
    files.dedup();
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
    if requires_orchestration_route(plugin_root, path)
        && !matches!(
            prompt_yaml::get_path(&parsed, &["interface", "default_prompt"]),
            Some(prompt_yaml::Scalar::Text(text)) if text.contains("$orchestration")
        )
    {
        errors.push(format!(
            "{} interface.default_prompt must route through $orchestration",
            display_relative(path)
        ));
    }
    let implicit_invocation =
        prompt_yaml::get_path(&parsed, &["policy", "allow_implicit_invocation"]);
    let valid_implicit_invocation = match implicit_invocation {
        Some(prompt_yaml::Scalar::Bool(true)) => true,
        Some(prompt_yaml::Scalar::Bool(false)) => is_explicit_only_core_skill(plugin_root, path),
        _ => false,
    };
    if !valid_implicit_invocation {
        errors.push(format!(
            "{} policy.allow_implicit_invocation must be true",
            display_relative(path)
        ));
    }
    Ok(errors)
}

fn requires_orchestration_route(plugin_root: &Path, path: &Path) -> bool {
    path == plugin_root.join("agents/openai.yaml")
        && plugin_name(plugin_root).as_deref() != Some("codexy-devtools")
}

fn is_explicit_only_core_skill(plugin_root: &Path, path: &Path) -> bool {
    plugin_name(plugin_root).as_deref() == Some("codexy")
        && ["realtime-voice-orchestration"].iter().any(|skill| {
            path == plugin_root
                .join("skills")
                .join(skill)
                .join("agents/openai.yaml")
        })
}

fn plugin_name(plugin_root: &Path) -> Option<String> {
    std::fs::read_to_string(plugin_root.join(".codex-plugin/plugin.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|manifest| manifest.get("name")?.as_str().map(str::to_owned))
}
