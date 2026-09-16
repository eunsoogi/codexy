use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::support::TestResult;

#[path = "readme_skills_support.rs"]
mod inventory;

#[test]
fn readmes_match_the_distributed_skill_inventory() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let expected = inventory::packaged(&root).map_err(std::io::Error::other)?;
    let english = std::fs::read_to_string(root.join("README.md"))?;
    let korean = std::fs::read_to_string(root.join("README.ko.md"))?;
    let english_rows = inventory::documented(&root, &english, &expected, "README.md")
        .map_err(std::io::Error::other)?;
    let korean_rows = inventory::documented(&root, &korean, &expected, "README.ko.md")
        .map_err(std::io::Error::other)?;

    if english_rows.keys().collect::<Vec<_>>() != korean_rows.keys().collect::<Vec<_>>() {
        return Err(std::io::Error::other("English and Korean inventory skill sets differ").into());
    }
    for (name, english_skill) in &english_rows {
        let korean_skill = korean_rows
            .get(name)
            .ok_or_else(|| std::io::Error::other(format!("Korean inventory misses {name}")))?;
        if english_skill.component != korean_skill.component
            || english_skill.link != korean_skill.link
        {
            return Err(std::io::Error::other(format!(
                "English and Korean inventory meaning differs for {name}"
            ))
            .into());
        }
    }
    inventory::guidance(&english, "English").map_err(std::io::Error::other)?;
    inventory::guidance(&korean, "Korean").map_err(std::io::Error::other)?;
    super::assert_local_links(&root, &root.join("README.md"), &english)
        .map_err(std::io::Error::other)?;
    super::assert_local_links(&root, &root.join("README.ko.md"), &korean)
        .map_err(std::io::Error::other)?;
    Ok(())
}

#[test]
fn readme_inventory_rejects_omissions_duplicates_components_and_links() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let expected = inventory::packaged(&root).map_err(std::io::Error::other)?;
    let english = std::fs::read_to_string(root.join("README.md"))?;
    let row = inventory::inventory_row(&english, "agents-md-authoring")
        .map_err(std::io::Error::other)?;

    let omission = english.replacen(&row, "", 1);
    assert!(inventory::documented(&root, &omission, &expected, "omission").is_err());

    let duplicate = english.replacen(&row, &format!("{row}\n{row}"), 1);
    assert!(inventory::documented(&root, &duplicate, &expected, "duplicate").is_err());

    let component = row.replacen("| `core` |", "| `github` |", 1);
    let component_error = english.replacen(&row, &component, 1);
    assert!(inventory::documented(&root, &component_error, &expected, "component").is_err());

    let link = row.replace(
        "plugins/codexy/skills/agents-md-authoring/SKILL.md",
        "plugins/codexy/skills/missing/SKILL.md",
    );
    let link_error = english.replacen(&row, &link, 1);
    assert!(inventory::documented(&root, &link_error, &expected, "link").is_err());
    Ok(())
}

#[test]
fn readme_skill_inventory_is_one_renderable_gfm_table_per_language() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let expected = inventory::packaged(&root).map_err(std::io::Error::other)?;
    for relative in ["README.md", "README.ko.md"] {
        let text = std::fs::read_to_string(root.join(relative))?;
        let table = inventory::inventory_table(&text).map_err(std::io::Error::other)?;
        let mut tables = 0;
        let mut columns = None;
        let mut headers = 0;
        let mut rows = 0;
        for event in Parser::new_ext(&table, Options::ENABLE_TABLES) {
            match event {
                Event::Start(Tag::Table(values)) => {
                    tables += 1;
                    columns = Some(values.len());
                }
                Event::Start(Tag::TableHead) => headers += 1,
                Event::Start(Tag::TableRow) => rows += 1,
                _ => {}
            }
        }
        assert_eq!(tables, 1, "{relative} inventory table count");
        assert_eq!(columns, Some(3), "{relative} inventory columns");
        assert_eq!(headers, 1, "{relative} inventory header count");
        assert_eq!(rows, expected.len(), "{relative} inventory data row count");
    }
    Ok(())
}

pub(super) fn link_count(text: &str, expected: &str) -> usize {
    text.split("](")
        .skip(1)
        .filter_map(|remainder| remainder.split(')').next())
        .filter(|target| *target == expected)
        .count()
}

pub(super) fn has_word(text: &str, expected: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| word.eq_ignore_ascii_case(expected))
}
