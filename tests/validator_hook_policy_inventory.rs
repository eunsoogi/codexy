use std::{collections::BTreeSet, fs, path::Path};

use regex::Regex;
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn inventory_exhaustively_classifies_normative_skill_rules() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
    let inventory: Value = serde_json::from_slice(&fs::read(root.join("hooks/policy-inventory.json"))?)?;
    let rules = inventory["rules"].as_array().ok_or("rules")?;
    assert_eq!(rules.len(), normative_count(&root.join("skills"))?);
    assert_eq!(inventory["summary"]["total"], rules.len());
    assert_eq!(inventory["summary"]["uncovered"], 0);
    let sources = rules.iter().map(|rule| rule["source"].as_str().ok_or("source")).collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(sources.len(), rules.len(), "stable source rows must be unique");
    for rule in rules {
        assert!(matches!(rule["decision"].as_str(), Some("enforced" | "reviewed-exception")));
        assert!(rule["positiveTests"].as_array().is_some_and(|items| !items.is_empty()));
        assert!(rule["negativeTests"].as_array().is_some_and(|items| !items.is_empty()));
    }
    Ok(())
}

#[test]
fn validator_rejects_missing_stale_and_unsupported_inventory_claims() -> TestResult {
    for mutate in ["missing", "stale-text", "unsupported-event"] {
        let fixture = tempfile::tempdir()?.keep();
        copy_tree(&Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"), &fixture)?;
        let path = fixture.join("hooks/policy-inventory.json");
        let mut inventory: Value = serde_json::from_slice(&fs::read(&path)?)?;
        let rules = inventory["rules"].as_array_mut().ok_or("rules")?;
        match mutate {
            "missing" => { rules.remove(0); }
            "stale-text" => rules[0]["text"] = Value::from("stale wording"),
            "unsupported-event" => rules[0]["event"] = Value::from("SessionStart"),
            _ => unreachable!(),
        }
        fs::write(&path, serde_json::to_vec(&inventory)?)?;
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--plugin-root", fixture.to_str().ok_or("fixture")?, "--check-hooks"])
            .output()?;
        assert!(!output.status.success(), "{mutate} claim must be rejected");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("inventory")
                || String::from_utf8_lossy(&output.stderr).contains("normative rule")
        );
    }
    Ok(())
}

#[test]
fn new_normative_rule_is_uncovered_until_explicitly_reviewed() -> TestResult {
    let fixture = tempfile::tempdir()?.keep();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &fixture,
    )?;
    let skill = fixture.join("skills/task-classification/SKILL.md");
    let mut text = fs::read_to_string(&skill)?;
    text.push_str("\nNew behavior MUST be reviewed before coverage.\n");
    fs::write(&skill, text)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            fixture.to_str().ok_or("fixture")?,
            "--check-hooks",
        ])
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uncovered normative rules"));
    Ok(())
}

#[test]
fn material_continuation_change_invalidates_reviewed_inventory() -> TestResult {
    let fixture = tempfile::tempdir()?.keep();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &fixture,
    )?;
    let skill = fixture.join("skills/agents-md-authoring/SKILL.md");
    let text = fs::read_to_string(&skill)?.replace(
        "filesystem root down through each ancestor directory to the target.",
        "repository root down through each ancestor directory to the target.",
    );
    fs::write(&skill, text)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", fixture.to_str().ok_or("fixture")?, "--check-hooks"])
        .output()?;
    assert!(!output.status.success(), "material continuation changes must invalidate the inventory");
    assert!(String::from_utf8_lossy(&output.stderr).contains("normative rule"));
    Ok(())
}

fn normative_count(root: &Path) -> TestResult<usize> {
    let mut files = Vec::new();
    collect_markdown(root, &mut files)?;
    let pattern = Regex::new(r"\bMUST(?: NOT)?\b")?;
    Ok(files
        .into_iter()
        .map(|path| {
            let mut fence: Option<(char, usize)> = None;
            fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|line| {
                    let marker = line.trim_start();
                    if let Some((character, length)) = fence {
                        let closing = marker.chars().take_while(|item| *item == character).count();
                        if closing >= length && marker[closing..].trim().is_empty() {
                            fence = None;
                        }
                        return 0;
                    }
                    let character = marker.chars().next();
                    let length = character.map_or(0, |item| marker.chars().take_while(|next| *next == item).count());
                    if length >= 3 && character.is_some_and(|item| matches!(item, '`' | '~')) {
                        fence = character.map(|item| (item, length));
                        0
                    } else {
                        pattern.find_iter(line).count()
                    }
                })
                .sum::<usize>()
        })
        .sum())
}

fn collect_markdown(root: &Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() { collect_markdown(&path, files)?; }
        else if path.extension().is_some_and(|extension| extension == "md") { files.push(path); }
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_tree(&entry.path(), &target)?; }
        else { fs::copy(entry.path(), target)?; }
    }
    Ok(())
}
