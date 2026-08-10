use std::path::Path;

use serde_json::Value;

use crate::support::TestResult;
use super::engineering_skill_fixture::copy_engineering_skill_fixture;

#[test]
fn production_validator_rejects_entrypoint_heading_mutations() -> TestResult {
    for mutation in [
        HeadingMutation::Removed,
        HeadingMutation::Substituted,
        HeadingMutation::Duplicated,
        HeadingMutation::FragmentSubstituted,
        HeadingMutation::Fenced,
        HeadingMutation::HtmlComment,
        HeadingMutation::RawHtml,
        HeadingMutation::InlineCode,
        HeadingMutation::Escaped,
        HeadingMutation::BlockQuote,
        HeadingMutation::OrderedList,
        HeadingMutation::UnorderedList,
        HeadingMutation::NestedContainers,
        HeadingMutation::H1TerminatesSection,
    ] {
        let (_temporary, plugin_root) = copy_engineering_skill_fixture()?;
        mutate_heading(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("heading {mutation:?}"));
    }
    Ok(())
}

#[test]
fn production_validator_rejects_inactive_destination_link_mutations() -> TestResult {
    for mutation in [
        LinkMutation::Fenced,
        LinkMutation::HtmlComment,
        LinkMutation::RawHtml,
        LinkMutation::InlineCode,
        LinkMutation::Escaped,
        LinkMutation::Image,
        LinkMutation::BlockQuote,
        LinkMutation::DuplicateElsewhere,
    ] {
        let (_temporary, plugin_root) = copy_engineering_skill_fixture()?;
        mutate_link(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("link {mutation:?}"));
    }
    Ok(())
}

#[test]
fn production_validator_rejects_noncanonical_identity_file_mutations() -> TestResult {
    for mutation in [
        IdentityMutation::ParentEscape,
        IdentityMutation::Absolute,
        IdentityMutation::SiblingPath,
        IdentityMutation::SourceFileSubstitution,
    ] {
        let (_temporary, plugin_root) = copy_engineering_skill_fixture()?;
        mutate_identity_file(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("identity path {mutation:?}"));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum HeadingMutation {
    Removed,
    Substituted,
    Duplicated,
    FragmentSubstituted,
    Fenced,
    HtmlComment,
    RawHtml,
    InlineCode,
    Escaped,
    BlockQuote,
    OrderedList,
    UnorderedList,
    NestedContainers,
    H1TerminatesSection,
}

#[derive(Clone, Copy, Debug)]
enum LinkMutation {
    Fenced,
    HtmlComment,
    RawHtml,
    InlineCode,
    Escaped,
    Image,
    BlockQuote,
    DuplicateElsewhere,
}

#[derive(Clone, Copy, Debug)]
enum IdentityMutation {
    ParentEscape,
    Absolute,
    SiblingPath,
    SourceFileSubstitution,
}

fn mutate_heading(plugin_root: &Path, mutation: HeadingMutation) -> TestResult {
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    match mutation {
        HeadingMutation::Removed => {
            let skill = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, skill.replacen("## Diagnosis\n", "", 1))?;
        }
        HeadingMutation::Substituted => {
            let skill = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, skill.replacen("## Diagnosis", "## Investigate", 1))?;
        }
        HeadingMutation::Duplicated => {
            let mut skill = std::fs::read_to_string(&skill_path)?;
            skill.push_str("\n## Diagnosis\n\nDuplicate heading.\n");
            std::fs::write(&skill_path, skill)?;
        }
        HeadingMutation::FragmentSubstituted => mutate_manifest(plugin_root, |mapping| {
            mapping["entrypoint"] = Value::String("SKILL.md#specification".to_owned());
        })?,
        HeadingMutation::Fenced => replace(&skill_path, "## Diagnosis", "```markdown\n## Diagnosis\n```")?,
        HeadingMutation::HtmlComment => {
            replace(&skill_path, "## Diagnosis", "<!--\n## Diagnosis\n-->")?
        }
        HeadingMutation::RawHtml => {
            replace(&skill_path, "## Diagnosis", "<div>\n## Diagnosis\n</div>")?
        }
        HeadingMutation::InlineCode => replace(&skill_path, "## Diagnosis", "`## Diagnosis`")?,
        HeadingMutation::Escaped => replace(&skill_path, "## Diagnosis", "\\## Diagnosis")?,
        HeadingMutation::BlockQuote => replace(&skill_path, "## Diagnosis", "> ## Diagnosis")?,
        HeadingMutation::OrderedList => replace(&skill_path, "## Diagnosis", "1. ## Diagnosis")?,
        HeadingMutation::UnorderedList => replace(&skill_path, "## Diagnosis", "- ## Diagnosis")?,
        HeadingMutation::NestedContainers => {
            replace(&skill_path, "## Diagnosis", "> - ## Diagnosis")?
        }
        HeadingMutation::H1TerminatesSection => replace(
            &skill_path,
            "MUST use [Diagnosis](references/diagnosis.md)",
            "# Intervening\n\nMUST use [Diagnosis](references/diagnosis.md)",
        )?,
    }
    Ok(())
}

fn mutate_link(plugin_root: &Path, mutation: LinkMutation) -> TestResult {
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let link = "[Diagnosis](references/diagnosis.md)";
    let whole_clause = "MUST use [Diagnosis](references/diagnosis.md)";
    match mutation {
        LinkMutation::RawHtml => {
            return replace(
                &skill_path,
                whole_clause,
                "<div>\nMUST use [Diagnosis](references/diagnosis.md)\n</div>",
            );
        }
        LinkMutation::BlockQuote => {
            return replace(
                &skill_path,
                whole_clause,
                "> MUST use [Diagnosis](references/diagnosis.md)",
            );
        }
        _ => {}
    }
    let replacement = match mutation {
        LinkMutation::Fenced => "```markdown\nMUST use [Diagnosis](references/diagnosis.md)\n```",
        LinkMutation::HtmlComment => "<!-- MUST use [Diagnosis](references/diagnosis.md) -->",
        LinkMutation::RawHtml => link,
        LinkMutation::InlineCode => "`[Diagnosis](references/diagnosis.md)`",
        LinkMutation::Escaped => "\\[Diagnosis\\](references/diagnosis.md)",
        LinkMutation::Image => "![Diagnosis](references/diagnosis.md)",
        LinkMutation::BlockQuote => link,
        LinkMutation::DuplicateElsewhere => {
            return append(&skill_path, "\n## Elsewhere\n\n[Diagnosis](references/diagnosis.md)\n");
        }
    };
    replace(&skill_path, link, replacement)
}

fn append(path: &Path, suffix: &str) -> TestResult {
    let skill = std::fs::read_to_string(path)?;
    std::fs::write(path, format!("{skill}{suffix}"))?;
    Ok(())
}

fn replace(path: &Path, needle: &str, replacement: &str) -> TestResult {
    let skill = std::fs::read_to_string(path)?;
    let changed = skill.replacen(needle, replacement, 1);
    assert_ne!(skill, changed, "fixture mutation must change the skill");
    std::fs::write(path, changed)?;
    Ok(())
}

fn mutate_identity_file(plugin_root: &Path, mutation: IdentityMutation) -> TestResult {
    let references = plugin_root.join("skills/engineering/references");
    let value = match mutation {
        IdentityMutation::ParentEscape => "../references/legacy-rule-mappings/debugging.json".to_owned(),
        IdentityMutation::Absolute => references
            .join("legacy-rule-mappings/debugging.json")
            .display()
            .to_string(),
        IdentityMutation::SiblingPath => "legacy-rule-mappings/../legacy-rule-mappings/debugging.json".to_owned(),
        IdentityMutation::SourceFileSubstitution => {
            std::fs::copy(
                references.join("legacy-rule-mappings/debugging.json"),
                references.join("identity-copy.json"),
            )?;
            "identity-copy.json".to_owned()
        }
    };
    mutate_manifest(plugin_root, |mapping| {
        mapping["identity_file"] = Value::String(value);
    })
}

fn mutate_manifest(plugin_root: &Path, mutate: impl FnOnce(&mut Value)) -> TestResult {
    let path = plugin_root.join("skills/engineering/references/legacy-rule-manifest.json");
    let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let mapping = manifest["mappings"]
        .as_array_mut()
        .and_then(|mappings| mappings.first_mut())
        .ok_or("debugging mapping missing")?;
    mutate(mapping);
    std::fs::write(path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(())
}

fn assert_rejected(plugin_root: &Path, label: &str) {
    let diagnostics = codexy_runtime::validation::engineering_equivalence_diagnostics(plugin_root);
    assert!(!diagnostics.is_empty(), "{label} must fail");
}
