use super::section;
use std::{collections::BTreeSet, path::Path};

pub(super) const DOCUMENTS: &[&str] = &[
    "docs/architecture.md",
    "docs/architecture/skill-boundaries.md",
    "docs/architecture/runtime-boundaries.md",
];

pub(super) fn combined(root: &Path) -> Result<String, std::io::Error> {
    DOCUMENTS
        .iter()
        .map(|path| std::fs::read_to_string(root.join(path)))
        .collect::<Result<Vec<_>, _>>()
        .map(|documents| documents.join("\n"))
}

pub(super) fn packaged_skills(root: &Path) -> Result<BTreeSet<String>, String> {
    let skills_root = root.join("plugins/codexy/skills");
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(skills_root).map_err(|error| error.to_string())? {
        let path = entry
            .map_err(|error| error.to_string())?
            .path()
            .join("SKILL.md");
        if !path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let frontmatter = text
            .split("---")
            .nth(1)
            .ok_or("skill frontmatter missing")?;
        let value: serde_yaml::Value =
            serde_yaml::from_str(frontmatter).map_err(|error| error.to_string())?;
        let name = value["name"]
            .as_str()
            .ok_or("skill name missing")?
            .to_owned();
        if !names.insert(name.clone()) {
            return Err(format!("duplicate packaged skill: {name}"));
        }
    }
    Ok(names)
}

pub(super) fn skill_path_consumer_count(guide: &str) -> Result<usize, String> {
    let section = section(guide, "Skill path-consumer map")?;
    let sentence = section
        .lines()
        .find(|line| line.starts_with("All "))
        .ok_or("missing skill path-consumer count")?;
    sentence
        .strip_prefix("All ")
        .and_then(|line| line.split_whitespace().next())
        .ok_or("missing skill path-consumer number")?
        .parse()
        .map_err(|error| format!("invalid skill path-consumer count: {error}"))
}

pub(super) fn assert_local_links(root: &Path, source: &Path, text: &str) -> Result<(), String> {
    let base = source.parent().ok_or("document has no parent")?;
    for remainder in text.split("](").skip(1) {
        let target = remainder.split(')').next().ok_or("unterminated link")?;
        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with('#')
        {
            continue;
        }
        let relative = target.split('#').next().unwrap_or(target);
        let path = base.join(relative);
        if !path.exists() {
            return Err(format!(
                "broken local link from {} to {} (repository {})",
                source.display(),
                path.display(),
                root.display()
            ));
        }
    }
    Ok(())
}
