use std::collections::BTreeMap;

use anyhow::{Context as _, Result, bail};
use regex::Regex;
use serde_yaml::Value;

#[derive(Debug)]
pub(super) struct Scalar {
    pub(super) line: usize,
    pub(super) text: String,
}

pub(super) fn parse(lines: &[&str]) -> Result<(Vec<Scalar>, usize)> {
    if lines.first().is_none_or(|line| line.trim() != "---") {
        return Ok((Vec::new(), 0));
    }
    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line.trim() == "---").then_some(index))
        .ok_or_else(|| anyhow::anyhow!("unterminated YAML frontmatter"))?;
    let yaml = lines[1..end].join("\n");
    let key_pattern =
        Regex::new(r"^([A-Za-z_][A-Za-z0-9_-]*)\s*:").expect("static YAML key pattern");
    let mut locations = BTreeMap::new();
    for (index, line) in lines[1..end].iter().enumerate() {
        if line.starts_with(char::is_whitespace)
            || line.trim().is_empty()
            || line.trim_start().starts_with('#')
        {
            continue;
        }
        let captures = key_pattern.captures(line).ok_or_else(|| {
            anyhow::anyhow!(
                "frontmatter key on line {} lacks a stable plain-key location",
                index + 2
            )
        })?;
        let key = captures[1].to_owned();
        if locations.insert(key.clone(), index + 2).is_some() {
            bail!("duplicate YAML frontmatter key {key:?}");
        }
    }
    let parsed: Value = serde_yaml::from_str(&yaml).context("parsing YAML frontmatter")?;
    let mapping = match parsed {
        Value::Null => return Ok((Vec::new(), end + 1)),
        Value::Mapping(mapping) => mapping,
        _ => bail!("YAML frontmatter must be a mapping"),
    };
    let mut scalars = Vec::new();
    for (key, line) in locations {
        let value = mapping
            .get(Value::String(key.clone()))
            .ok_or_else(|| anyhow::anyhow!("frontmatter key {key:?} was not parsed"))?;
        if let Value::String(text) = value {
            scalars.push(Scalar {
                line,
                text: text.clone(),
            });
        }
    }
    Ok((scalars, end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_and_duplicate_yaml_but_accepts_empty_scalars() -> Result<()> {
        assert!(parse(&["---", "description: [invalid", "---"]).is_err());
        assert!(
            parse(&[
                "---",
                "description: first MUST apply",
                "description: second MUST apply",
                "---",
            ])
            .is_err()
        );
        let (scalars, body) = parse(&["---", "name: example", "description:", "---"])?;
        assert_eq!(body, 4);
        assert_eq!(scalars.len(), 1);
        assert_eq!(scalars[0].text, "example");
        Ok(())
    }
}
