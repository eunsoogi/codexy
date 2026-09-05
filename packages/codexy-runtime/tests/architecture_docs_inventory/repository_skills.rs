use std::{collections::BTreeSet, path::Path};

use crate::support::TestResult;

use super::{exact_names, rows};

const PROJECT_SKILLS: &[&str] = &[
    "plugin-marketplace-prep",
    "release-engineering",
    "skill-evaluation",
];

#[test]
fn repository_only_skills_are_discoverable_without_plugin_exposure() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skills_root = root.join(".agents/skills");
    let documented = std::fs::read_to_string(root.join("docs/architecture.md"))?;
    let expected = PROJECT_SKILLS.iter().map(|skill| (*skill).to_owned()).collect::<BTreeSet<_>>();
    let discovered = std::fs::read_dir(&skills_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("SKILL.md"))
        .filter(|path| path.is_file())
        .map(|path| skill_name(&path))
        .collect::<Result<BTreeSet<_>, _>>()?;

    assert_eq!(discovered, expected);
    exact_names(&rows(&documented, "Repository-only skills")?, &expected, "repository skill", 4)?;

    for skill in PROJECT_SKILLS {
        assert!(skills_root.join(skill).join("SKILL.md").is_file());
        assert!(skills_root.join(skill).join("agents/openai.yaml").is_file());
        assert!(!root
            .join("plugins/codexy/skills")
            .join(skill)
            .exists());
    }
    Ok(())
}

fn skill_name(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let frontmatter = text.split("---").nth(1).ok_or("skill frontmatter missing")?;
    let value: serde_yaml::Value = serde_yaml::from_str(frontmatter)?;
    Ok(value["name"].as_str().ok_or("skill name missing")?.to_owned())
}
