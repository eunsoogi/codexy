use std::path::{Component, Path, PathBuf};

use super::active_markdown::Document;
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
    let document = Document::parse(skill);
    let Some(heading) = document.unique_heading(anchor) else {
        return Err(format!(
            "entrypoint anchor {anchor} must resolve to one heading"
        ));
    };
    let section = &skill[heading.start..heading.end];
    let target = format!("references/{destination}");
    if !document.exact_top_level_link(heading, &target) {
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
