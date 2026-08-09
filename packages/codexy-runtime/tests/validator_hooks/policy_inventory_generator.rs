use std::{fs, path::{Path, PathBuf}, process::Command};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CAPABILITY_EVIDENCE: &str =
    "codexy.hooks.capability-contract content digest 3f12e358c360fce1";

#[test]
fn generator_imports_a_reviewed_decision_and_preserves_it_without_input() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = fixture(temp.path())?;
    assert!(!generate(&root, None)?.status.success());
    let discovered = discovered_rule(&root)?;
    let decisions = root.join("decisions.json");
    fs::write(&decisions, serde_json::to_vec(&decision(&discovered))?)?;

    let output = generate(&root, Some(&decisions))?;
    assert!(output.status.success(), "{}", text(&output));
    let imported = inventory_rule(&root, &discovered["digest"])?;
    assert_eq!(imported["decision"], "reviewed-exception");
    assert_eq!(imported["tests"], json!(["thread-routing"]));
    assert!(imported["evidence"]
        .as_array()
        .ok_or("evidence")?
        .iter()
        .any(|entry| entry == CAPABILITY_EVIDENCE));
    assert!(imported["evidence"]
        .as_array()
        .ok_or("evidence")?
        .iter()
        .any(|entry| entry == "exact focused test"));

    let first = fs::read(root.join("plugins/codexy/hooks/policy-inventory.json"))?;
    assert!(generate(&root, None)?.status.success());
    assert_eq!(fs::read(root.join("plugins/codexy/hooks/policy-inventory.json"))?, first);
    Ok(())
}

#[test]
fn generator_rejects_untrusted_or_incomplete_review_decisions() -> TestResult {
    for case in [
        "unknown",
        "stale",
        "duplicate",
        "identity",
        "decision",
        "event",
        "input",
        "positive",
        "negative",
        "evidence",
    ] {
        let temp = tempfile::tempdir()?;
        let root = fixture(temp.path())?;
        assert!(!generate(&root, None)?.status.success());
        let discovered = discovered_rule(&root)?;
        let mut input = decision(&discovered);
        mutate(&mut input, case);
        let decisions = root.join("decisions.json");
        fs::write(&decisions, serde_json::to_vec(&input)?)?;
        let output = generate(&root, Some(&decisions))?;
        assert!(!output.status.success(), "{case} unexpectedly passed: {}", text(&output));
    }
    Ok(())
}

#[test]
fn fixture_sources_checked_in_assets_from_repository_root() -> TestResult {
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();
    assert_ne!(repository, runtime.as_path());
    assert!(repository.join("plugins/codexy/hooks/capability-contract.json").is_file());
    assert!(repository.join("scripts/generate-hook-policy-inventory").is_file());
    assert!(!runtime.join("plugins/codexy/hooks/capability-contract.json").exists());
    assert!(!runtime.join("scripts/generate-hook-policy-inventory").exists());

    let temp = tempfile::tempdir()?;
    let root = fixture(temp.path())?;
    assert_eq!(
        fs::read(root.join("plugins/codexy/hooks/capability-contract.json"))?,
        fs::read(repository.join("plugins/codexy/hooks/capability-contract.json"))?
    );
    assert_eq!(
        fs::read(root.join("scripts/generate-hook-policy-inventory"))?,
        fs::read(repository.join("scripts/generate-hook-policy-inventory"))?
    );
    Ok(())
}

#[test]
fn fixture_missing_generator_does_not_resolve_an_unrelated_executable() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = fixture(temp.path())?;
    fs::remove_file(root.join("scripts/generate-hook-policy-inventory"))?;
    assert!(!generate(&root, None)?.status.success());
    Ok(())
}

fn fixture(base: &Path) -> TestResult<PathBuf> {
    let root = base.join("repository");
    let plugin = root.join("plugins/codexy");
    fs::create_dir_all(plugin.join("skills/fixture"))?;
    fs::create_dir_all(plugin.join("hooks"))?;
    fs::create_dir_all(root.join("scripts"))?;
    fs::write(
        plugin.join("skills/fixture/SKILL.md"),
        "# Fixture\n\nThe fixture owner MUST retain the reviewed decision.\n",
    )?;
    let repository = codexy_runtime::paths::repository_root();
    fs::copy(
        repository.join("plugins/codexy/hooks/capability-contract.json"),
        plugin.join("hooks/capability-contract.json"),
    )?;
    fs::copy(
        repository.join("scripts/generate-hook-policy-inventory"),
        root.join("scripts/generate-hook-policy-inventory"),
    )?;
    fs::copy(
        repository.join("scripts/policy_inventory_review_decisions.py"),
        root.join("scripts/policy_inventory_review_decisions.py"),
    )?;
    Ok(root)
}

fn generate(root: &Path, decisions: Option<&Path>) -> TestResult<std::process::Output> {
    let mut command = Command::new("python3");
    command.arg(root.join("scripts/generate-hook-policy-inventory"));
    if let Some(decisions) = decisions {
        command.args(["--review-decisions-file", decisions.to_str().ok_or("decisions path")?]);
    }
    Ok(command.output()?)
}

fn discovered_rule(root: &Path) -> TestResult<Value> {
    let inventory: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy/hooks/policy-inventory.json"),
    )?)?;
    inventory["rules"]
        .as_array()
        .and_then(|rules| rules.first())
        .cloned()
        .ok_or_else(|| "generated rule".into())
}

fn inventory_rule(root: &Path, digest: &Value) -> TestResult<Value> {
    let inventory: Value = serde_json::from_slice(&fs::read(
        root.join("plugins/codexy/hooks/policy-inventory.json"),
    )?)?;
    inventory["rules"]
        .as_array()
        .and_then(|rules| rules.iter().find(|rule| rule["digest"] == *digest))
        .cloned()
        .ok_or_else(|| "imported rule".into())
}

fn decision(rule: &Value) -> Value {
    json!({
        "schema": "codexy.hooks.review-decisions",
        "decisions": [{
            "digest": rule["digest"], "text": rule["text"],
            "event": "unavailable", "input": "unavailable", "decision": "reviewed-exception",
            "tests": ["thread-routing"], "evidence": ["exact focused test"],
            "positiveTests": ["thread-routing"], "negativeTests": ["thread-routing"],
            "unavailableEvent": "No official hook event exposes the ordered state.",
            "unavailableInput": "No official hook input exposes the ordered state.",
            "rationale": "The focused validator proves the structural boundary."
        }]
    })
}

fn mutate(input: &mut Value, case: &str) {
    let decision = &mut input["decisions"][0];
    match case {
        "unknown" => decision["digest"] = json!("0000000000000000"),
        "stale" => decision["text"] = json!("wrong text"),
        "duplicate" => {
            let duplicate = decision.clone();
            input["decisions"].as_array_mut().unwrap().push(duplicate);
        }
        "identity" => decision["id"] = json!("must-not-override"),
        "decision" => decision["decision"] = json!("enforced"),
        "event" => decision["event"] = json!("PreToolUse"),
        "input" => decision["input"] = json!("bash-command"),
        "positive" => {
            decision.as_object_mut().unwrap().remove("positiveTests");
        }
        "negative" => {
            decision.as_object_mut().unwrap().remove("negativeTests");
        }
        "evidence" => {
            decision.as_object_mut().unwrap().remove("evidence");
        }
        _ => unreachable!(),
    }
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
