use std::path::{Component, Path, PathBuf};

use regex::Regex;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(super) struct Semantic {
    pub(super) id: String,
    pub(super) value: String,
}

pub(super) fn trigger(text: &str) -> Result<String, String> {
    let frontmatter = text
        .split("---")
        .nth(1)
        .ok_or("baseline frontmatter missing")?;
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(frontmatter).map_err(|error| error.to_string())?;
    let description = yaml["description"]
        .as_str()
        .ok_or("baseline description missing")?;
    description
        .strip_prefix("MUST use when ")
        .map(ToOwned::to_owned)
        .ok_or_else(|| "baseline description must start with `MUST use when`".to_owned())
}

pub(super) fn identities(name: &str, text: &str, source_path: &Path) -> Vec<Semantic> {
    let body = text.splitn(3, "---").nth(2).unwrap_or_default();
    let mut values = trigger(text).into_iter().collect::<Vec<_>>();
    values.extend(blocks(body));
    values.extend(fences(body));
    values.extend(references(body));
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let value = normalize(&value, source_path);
            let kind = if index == 0 { "trigger" } else { "semantic" };
            Semantic {
                id: format!("{name}:{kind}:{index}:{}", hash(&value)),
                value,
            }
        })
        .collect()
}

pub(super) fn destination_values(text: &str, path: &Path) -> Vec<String> {
    let mut values = blocks(text);
    values.extend(fences(text));
    values.extend(references(text));
    values
        .into_iter()
        .map(|value| normalize(&value, path))
        .collect()
}

pub(super) fn normalized_text(text: &str, path: &Path) -> String {
    normalize(text, path)
}

pub(super) fn local_links(text: &str, path: &Path, plugin_root: &Path) -> Vec<String> {
    link_regex()
        .captures_iter(text)
        .filter_map(|captures| {
            let target = captures.get(2)?.as_str();
            if target.starts_with('#') || target.contains("://") || target.starts_with("mailto:") {
                return None;
            }
            Some(resolve(path, target, plugin_root).map_or_else(
                |error| format!("{}: {error}", path.display()),
                |resolved| {
                    if resolved.is_file() {
                        String::new()
                    } else {
                        format!("{}: missing local link {target}", path.display())
                    }
                },
            ))
        })
        .filter(|error| !error.is_empty())
        .collect()
}

fn blocks(body: &str) -> Vec<String> {
    let normalized = body.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        let boundary = trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("```")
            || marker(trimmed);
        if boundary && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        }
        if marker(trimmed) {
            current.push(trimmed.to_owned());
        } else if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("```") {
            current.push(trimmed.to_owned());
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn fences(body: &str) -> Vec<String> {
    let mut inside = false;
    let mut current = Vec::new();
    let mut fences = Vec::new();
    for line in body.replace("\r\n", "\n").lines() {
        if line.starts_with("```") {
            current.push(line.to_owned());
            inside = !inside;
            if !inside {
                fences.push(current.join("\n"));
                current.clear();
            }
        } else if inside {
            current.push(line.to_owned());
        }
    }
    fences
}

fn references(body: &str) -> Vec<String> {
    link_regex()
        .captures_iter(body)
        .filter_map(|captures| {
            Some(format!(
                "[{}]({})",
                captures.get(1)?.as_str(),
                captures.get(2)?.as_str()
            ))
        })
        .collect()
}

fn marker(line: &str) -> bool {
    line.starts_with("- ")
        || line.chars().take_while(char::is_ascii_digit).count() > 0 && line.contains(". ")
}

fn normalize(value: &str, path: &Path) -> String {
    let soft = value
        .replace("\r\n", "\n")
        .split('\n')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    link_regex()
        .replace_all(&soft, |captures: &regex::Captures<'_>| {
            let label = captures.get(1).map_or("", |item| item.as_str());
            let target = captures.get(2).map_or("", |item| item.as_str());
            format!("[{label}]({})", canonical_target(path, target))
        })
        .into_owned()
}

fn canonical_target(path: &Path, target: &str) -> String {
    if target.starts_with('#') || target.contains("://") || target.starts_with("mailto:") {
        return target.to_owned();
    }
    let resolved = normalize_path(path.parent().unwrap_or(path).join(target));
    let mut components = resolved
        .components()
        .skip_while(|component| component.as_os_str() != "skills");
    let relative = components.by_ref().collect::<PathBuf>();
    if relative.as_os_str().is_empty() {
        resolved.display().to_string()
    } else {
        relative
            .display()
            .to_string()
            .replace("skills/codex-orchestration/", "skills/orchestration/")
    }
}

fn resolve(path: &Path, target: &str, plugin_root: &Path) -> Result<PathBuf, String> {
    let resolved = normalize_path(
        path.parent()
            .ok_or("link source parent missing")?
            .join(target),
    );
    if !resolved.starts_with(plugin_root) {
        return Err(format!("local link escapes plugin root: {target}"));
    }
    Ok(resolved)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::ParentDir => {
                    normalized.pop();
                }
                Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
            normalized
        })
}

fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn link_regex() -> &'static Regex {
    static LINK: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    LINK.get_or_init(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").expect("valid link regex"))
}
