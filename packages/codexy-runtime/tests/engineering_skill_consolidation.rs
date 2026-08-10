use std::path::Path;

use serde_json::Value;

use crate::support::{TestResult, copy_plugin_fixture};

#[test]
fn production_validator_accepts_the_real_engineering_route_and_mode_all() -> TestResult {
    let (_temporary, plugin_root) = copy_plugin_fixture()?;

    let diagnostics = codexy_runtime::validation::engineering_equivalence_diagnostics(&plugin_root);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let all = codexy_runtime::validation::errors(&plugin_root, codexy_runtime::validation::Mode::All);
    assert!(all.is_empty(), "{all:#?}");
    Ok(())
}

#[test]
fn production_validator_rejects_baseline_and_source_inventory_mutations() -> TestResult {
    let sources = codexy_runtime::validation::engineering_equivalence_baseline_sources();
    assert!(codexy_runtime::validation::engineering_equivalence_baseline_diagnostics(&sources).is_empty());

    for mutation in [
        BaselineMutation::Bytes,
        BaselineMutation::Missing,
        BaselineMutation::Extra,
        BaselineMutation::Duplicate,
        BaselineMutation::Unknown,
    ] {
        let mut changed = sources.clone();
        apply_baseline_mutation(&mut changed, mutation);
        assert!(
            !codexy_runtime::validation::engineering_equivalence_baseline_diagnostics(&changed).is_empty(),
            "baseline mutation {mutation:?} must fail"
        );
    }
    Ok(())
}

#[test]
fn production_validator_rejects_manifest_projection_mutations() -> TestResult {
    for mutation in [
        ManifestMutation::MissingRule,
        ManifestMutation::ExtraRule,
        ManifestMutation::DuplicateRule,
        ManifestMutation::UnknownRule,
        ManifestMutation::StaleSource,
        ManifestMutation::TwiceMappedRule,
        ManifestMutation::DuplicateDestination,
        ManifestMutation::MissingDestination,
        ManifestMutation::UnlinkedDestination,
    ] {
        let (_temporary, plugin_root) = copy_plugin_fixture()?;
        apply_manifest_mutation(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("manifest {mutation:?}"));
    }
    Ok(())
}

#[test]
fn production_validator_rejects_destination_and_trigger_mutations() -> TestResult {
    for mutation in [
        TextMutation::MustToMay,
        TextMutation::Negation,
        TextMutation::Lexical,
        TextMutation::TriggerRemoval,
        TextMutation::TriggerSubstitution,
        TextMutation::BrokenLink,
        TextMutation::OutsideLink,
    ] {
        let (_temporary, plugin_root) = copy_plugin_fixture()?;
        apply_text_mutation(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("text {mutation:?}"));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum BaselineMutation { Bytes, Missing, Extra, Duplicate, Unknown }

#[derive(Clone, Copy, Debug)]
enum ManifestMutation {
    MissingRule, ExtraRule, DuplicateRule, UnknownRule, StaleSource, TwiceMappedRule,
    DuplicateDestination, MissingDestination, UnlinkedDestination,
}

#[derive(Clone, Copy, Debug)]
enum TextMutation {
    MustToMay, Negation, Lexical, TriggerRemoval, TriggerSubstitution, BrokenLink, OutsideLink,
}

fn apply_baseline_mutation(sources: &mut Vec<(String, String)>, mutation: BaselineMutation) {
    match mutation {
        BaselineMutation::Bytes => sources[0].1.push('x'),
        BaselineMutation::Missing => { sources.pop(); }
        BaselineMutation::Extra => sources.push(("unexpected".to_owned(), "bytes".to_owned())),
        BaselineMutation::Duplicate => sources[1].0 = sources[0].0.clone(),
        BaselineMutation::Unknown => sources[0].0 = "unknown".to_owned(),
    }
}

fn apply_manifest_mutation(plugin_root: &Path, mutation: ManifestMutation) -> TestResult {
    let manifest_path = plugin_root.join("skills/engineering/references/legacy-rule-manifest.json");
    let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let mappings = manifest["mappings"].as_array_mut().ok_or("manifest mappings missing")?;
    match mutation {
        ManifestMutation::DuplicateDestination => mappings[1]["destination"] = mappings[0]["destination"].clone(),
        ManifestMutation::MissingDestination => mappings[0]["destination"] = Value::String("missing.md".to_owned()),
        ManifestMutation::UnlinkedDestination => mappings[0]["entrypoint"] = Value::String("SKILL.md#missing".to_owned()),
        ManifestMutation::StaleSource => mappings[0]["source"] = Value::String("debugging-v0".to_owned()),
        _ => mutate_identity_file(plugin_root, mappings[0]["identity_file"].as_str().ok_or("identity file missing")?, mutation)?,
    }
    std::fs::write(manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(())
}

fn mutate_identity_file(plugin_root: &Path, relative: &str, mutation: ManifestMutation) -> TestResult {
    let path = plugin_root.join("skills/engineering/references").join(relative);
    let mut value: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let identities = value["identities"].as_array_mut().ok_or("identities missing")?;
    match mutation {
        ManifestMutation::MissingRule => { identities.pop(); }
        ManifestMutation::ExtraRule => identities.push(Value::String("unexpected:rule".to_owned())),
        ManifestMutation::DuplicateRule => identities.push(identities[0].clone()),
        ManifestMutation::UnknownRule => identities[0] = Value::String("unknown:rule".to_owned()),
        ManifestMutation::TwiceMappedRule => identities.push(identities[1].clone()),
        ManifestMutation::StaleSource | ManifestMutation::DuplicateDestination | ManifestMutation::MissingDestination | ManifestMutation::UnlinkedDestination => unreachable!(),
    }
    std::fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn apply_text_mutation(plugin_root: &Path, mutation: TextMutation) -> TestResult {
    let skill = plugin_root.join("skills/engineering/SKILL.md");
    let first_destination = first_destination(plugin_root)?;
    let destination = plugin_root.join("skills/engineering/references").join(first_destination);
    let path = match mutation { TextMutation::TriggerRemoval | TextMutation::TriggerSubstitution => &skill, _ => &destination };
    let original = std::fs::read_to_string(path)?;
    let changed = match mutation {
        TextMutation::MustToMay => original.replacen("MUST", "MAY", 1),
        TextMutation::Negation => original.replacen("MUST NOT", "MUST", 1),
        TextMutation::Lexical => original.replacen("evidence", "guesswork", 1),
        TextMutation::TriggerRemoval => mutate_first_trigger(&original, "MUST use [", "Use ["),
        TextMutation::TriggerSubstitution => mutate_first_trigger(&original, "behavior", "ceremony"),
        TextMutation::BrokenLink => original.replacen(".md)", "-missing.md)", 1),
        TextMutation::OutsideLink => original.replacen(
            "](../../codex-orchestration/references/plain-language-user-replies.md)",
            "](../../../../AGENTS.md)",
            1,
        ),
    };
    std::fs::write(path, changed)?;
    Ok(())
}

fn mutate_first_trigger(original: &str, from: &str, to: &str) -> String {
    let offset = original.find("MUST use [").unwrap_or_default();
    let (prefix, route) = original.split_at(offset);
    format!("{prefix}{}", route.replacen(from, to, 1))
}

fn first_destination(plugin_root: &Path) -> TestResult<String> {
    let manifest = std::fs::read_to_string(plugin_root.join("skills/engineering/references/legacy-rule-manifest.json"))?;
    let value: Value = serde_json::from_str(&manifest)?;
    value["mappings"][0]["destination"].as_str().map(ToOwned::to_owned).ok_or_else(|| "destination missing".into())
}

fn assert_rejected(plugin_root: &Path, label: &str) {
    assert!(
        !codexy_runtime::validation::engineering_equivalence_diagnostics(plugin_root).is_empty(),
        "{label} must fail"
    );
}
