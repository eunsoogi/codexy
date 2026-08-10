use std::path::{Component, Path, PathBuf};

use regex::Regex;

use super::data::Mapping;

pub(super) fn entrypoint_section<'a>(
    mapping: &Mapping,
    destination: &str,
    anchor: &str,
    skill: &'a str,
) -> Result<&'a str, String> {
    let expected = format!("SKILL.md#{anchor}");
    let Some((entrypoint_file, fragment)) = mapping.entrypoint.split_once('#') else {
        return Err("mapping entrypoint must contain one Markdown fragment".to_owned());
    };
    if entrypoint_file != "SKILL.md"
        || fragment != anchor
        || mapping.entrypoint.matches('#').count() != 1
        || mapping.destination != destination
        || mapping.entrypoint != expected
    {
        return Err("mapping has a stale destination or entrypoint".to_owned());
    }
    let heading = headings(skill)
        .into_iter()
        .filter(|item| item.anchor == anchor)
        .collect::<Vec<_>>();
    if heading.len() != 1 {
        return Err(format!(
            "entrypoint anchor {anchor} must resolve to one heading"
        ));
    }
    let heading = &heading[0];
    let section = &skill[heading.start..heading.end];
    let target = format!("references/{destination}");
    if links(section)
        .iter()
        .filter(|value| **value == target)
        .count()
        != 1
    {
        return Err(format!(
            "entrypoint anchor {anchor} must link once to {target}"
        ));
    }
    Ok(section)
}

pub(super) fn canonical_identity_path(
    references: &Path,
    source: &str,
    supplied: &str,
) -> Result<PathBuf, String> {
    let expected = format!("legacy-rule-mappings/{source}.json");
    let supplied_path = Path::new(supplied);
    if supplied_path.is_absolute()
        || supplied_path.components().any(|item| {
            matches!(
                item,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "identity file for {source} escapes the canonical references root"
        ));
    }
    if supplied != expected {
        return Err(format!("identity file for {source} must be {expected}"));
    }
    let root = references
        .canonicalize()
        .map_err(|error| format!("canonical references root: {error}"))?;
    let path = references
        .join(supplied_path)
        .canonicalize()
        .map_err(|error| format!("canonical identity file for {source}: {error}"))?;
    if !path.starts_with(&root) {
        return Err(format!(
            "identity file for {source} escapes the canonical references root"
        ));
    }
    let expected_path = references
        .join(expected)
        .canonicalize()
        .map_err(|error| format!("canonical expected identity file for {source}: {error}"))?;
    if path != expected_path {
        return Err(format!(
            "identity file for {source} does not bind its canonical source file"
        ));
    }
    Ok(path)
}

struct Heading {
    anchor: String,
    start: usize,
    end: usize,
}

fn headings(text: &str) -> Vec<Heading> {
    let mut result = Vec::new();
    let mut offset = 0;
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            result.push(Heading {
                anchor: heading_anchor(title),
                start: offset,
                end: text.len(),
            });
        }
        offset += line.len() + 1;
    }
    for index in 0..result.len().saturating_sub(1) {
        result[index].end = result[index + 1].start;
    }
    result
}

fn heading_anchor(title: &str) -> String {
    let mut anchor = String::new();
    let mut separator = false;
    for character in title.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !anchor.is_empty() {
                anchor.push('-');
            }
            anchor.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    anchor
}

fn links(text: &str) -> Vec<&str> {
    static LINKS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    LINKS
        .get_or_init(|| Regex::new(r"\[[^\]]+\]\(([^)]+)\)").expect("valid link regex"))
        .captures_iter(text)
        .filter_map(|item| item.get(1).map(|target| target.as_str()))
        .collect()
}
