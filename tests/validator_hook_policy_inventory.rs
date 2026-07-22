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

fn normative_count(root: &Path) -> TestResult<usize> {
    let mut files = Vec::new();
    collect_markdown(root, &mut files)?;
    let pattern = Regex::new(r"\bMUST(?: NOT)?\b")?;
    Ok(files.into_iter().map(|path| pattern.find_iter(&fs::read_to_string(path).unwrap()).count()).sum())
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
