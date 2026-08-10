use serde_yaml::Value;

use super::wiki_minimal_contract_markdown::Document;

pub(crate) fn validate_core_skill(source: &str, removed: &[&str]) -> Result<(), String> {
    let document = Document::parse(source)?;
    let workflow = document.section("## Core workflow")?;
    required_count(
        document.inline_code_count(Some(&workflow), "init → ingest → compile → query → refresh"),
        "core command inventory",
    )?;
    required_count(
        document.link_count("Migration", "references/migration.md"),
        "migration link",
    )?;
    for command in removed {
        if document.inline_code_count(None, command) != 0 {
            return Err(format!("removed command remains active: {command}"));
        }
    }
    Ok(())
}

pub(crate) fn markdown_link_count(
    source: &str,
    label: &str,
    target: &str,
) -> Result<usize, String> {
    Ok(Document::parse(source)?.link_count(label, target))
}

pub(crate) fn frontmatter_string(source: &str, key: &str) -> Result<String, String> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, remainder) = source.split_once('\n').ok_or("frontmatter opening")?;
    if opening.trim_end_matches('\r') != "---" {
        return Err("frontmatter opening".into());
    }
    let mut end = 0;
    for line in remainder.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            break;
        }
        end += line.len();
    }
    let yaml = (end < remainder.len())
        .then_some(&remainder[..end])
        .ok_or("frontmatter closing")?;
    let Value::Mapping(mapping) =
        serde_yaml::from_str::<Value>(yaml).map_err(|error| error.to_string())?
    else {
        return Err("frontmatter mapping".into());
    };
    mapping
        .get(Value::String(key.into()))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("frontmatter string: {key}"))
}

fn required_count(count: usize, identity: &str) -> Result<(), String> {
    (count == 1)
        .then_some(())
        .ok_or_else(|| format!("missing or duplicate {identity}"))
}
