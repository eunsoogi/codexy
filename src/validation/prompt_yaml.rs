use std::{collections::BTreeMap, path::Path};

use anyhow::{Result, bail};

use crate::paths::display_relative;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Scalar {
    Bool(bool),
    Text(String),
    Map(BTreeMap<String, Self>),
}

pub(super) fn parse(text: &str, path: &Path) -> Result<BTreeMap<String, Scalar>> {
    let mut root = BTreeMap::new();
    let mut stack: Vec<(usize, Vec<String>)> = vec![(0, Vec::new())];
    let mut previous_indent = 0usize;
    let mut previous_was_map = true;
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
            continue;
        }
        let indent = raw_line.len() - raw_line.trim_start_matches(' ').len();
        if raw_line[..indent].contains('\t') {
            bail!(
                "{} must not contain tab indentation",
                display_relative(path)
            );
        }
        let stripped = raw_line.trim();
        if !stripped.contains(':') {
            bail!(
                "{} line {line_number} must be a YAML key/value pair",
                display_relative(path)
            );
        }
        if indent > previous_indent && !previous_was_map {
            bail!(
                "{} line {line_number} cannot be nested under a scalar value",
                display_relative(path)
            );
        }
        while stack.last().is_some_and(|(level, _)| indent < *level) {
            stack.pop();
        }
        let (key, raw_value) = stripped.split_once(':').unwrap_or((stripped, ""));
        let key = key.trim();
        if key.is_empty() {
            bail!(
                "{} line {line_number} has an empty key",
                display_relative(path)
            );
        }
        let mut path_keys = stack.last().map_or_else(Vec::new, |(_, keys)| keys.clone());
        set_value(
            &mut root,
            &path_keys,
            key,
            parse_value(raw_value.trim(), path)?,
        )?;
        if raw_value.trim().is_empty() {
            path_keys.push(key.to_owned());
            stack.push((indent + 2, path_keys));
            previous_was_map = true;
        } else {
            previous_was_map = false;
        }
        previous_indent = indent;
    }
    Ok(root)
}

pub(super) fn get_path<'a>(
    root: &'a BTreeMap<String, Scalar>,
    keys: &[&str],
) -> Option<&'a Scalar> {
    let (first, rest) = keys.split_first()?;
    let mut current = root.get(*first)?;
    for key in rest {
        let Scalar::Map(map) = current else {
            return None;
        };
        current = map.get(*key)?;
    }
    Some(current)
}

fn set_value(
    root: &mut BTreeMap<String, Scalar>,
    parents: &[String],
    key: &str,
    value: Scalar,
) -> Result<()> {
    let mut current = root;
    for parent in parents {
        let entry = current
            .entry(parent.clone())
            .or_insert_with(|| Scalar::Map(BTreeMap::new()));
        let Scalar::Map(map) = entry else {
            bail!("cannot nest under scalar YAML value");
        };
        current = map;
    }
    current.insert(key.to_owned(), value);
    Ok(())
}

fn parse_value(value: &str, path: &Path) -> Result<Scalar> {
    if value.is_empty() {
        return Ok(Scalar::Map(BTreeMap::new()));
    }
    if value == "true" {
        return Ok(Scalar::Bool(true));
    }
    if value == "false" {
        return Ok(Scalar::Bool(false));
    }
    let starts_quote = value.starts_with('"') || value.starts_with('\'');
    let ends_quote = value.ends_with('"') || value.ends_with('\'');
    if starts_quote != ends_quote {
        bail!("{} quoted scalar is unterminated", display_relative(path));
    }
    if starts_quote {
        Ok(Scalar::Text(
            value[1..value.len().saturating_sub(1)].to_owned(),
        ))
    } else {
        Ok(Scalar::Text(value.to_owned()))
    }
}
