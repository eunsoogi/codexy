use std::{collections::BTreeSet, path::Path, process::Command};

use serde_json::json;

use crate::support::{TestResult, copy_plugin_fixture};

const CURRENT: [&str; 8] = [
    "codexy-architect",
    "codexy-auditor",
    "codexy-cartographer",
    "codexy-inspector",
    "codexy-sentinel",
    "codexy-shipwright",
    "codexy-warden",
    "codexy-watcher",
];
const RETIRED: [&str; 5] = [
    "codexy-forge",
    "codexy-pathfinder",
    "codexy-scribe",
    "codexy-sculptor",
    "codexy-tracer",
];

#[test]
fn current_catalog_matches_expected_specialists() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    assert_current_catalog(&plugin_root)
}

#[test]
fn retired_specialists_are_not_registered_or_callable() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    assert_retired_specialists(&plugin_root)
}

#[test]
fn historical_prose_does_not_define_current_role_contract() -> TestResult {
    let (temp, plugin_root) = copy_plugin_fixture()?;
    let docs_root = temp.path().join("docs");
    std::fs::create_dir_all(&docs_root)?;
    std::fs::write(
        docs_root.join("specialist-role-equivalence.md"),
        "The retired `codexy-forge` role remains documented for migration history.\n",
    )?;
    for path in [
        plugin_root.join("skills/orchestration/SKILL.md"),
        plugin_root.join("skills/orchestration/references/classification-and-control.md"),
    ] {
        let mut contents = std::fs::read_to_string(&path)?;
        contents.push_str(
            "\nHistorical note: `codexy-forge` is retired and is not part of active routing.\n",
        );
        std::fs::write(path, contents)?;
    }

    assert_current_catalog(&plugin_root)?;
    assert_retired_specialists(&plugin_root)
}

#[test]
fn disposable_catalog_mutations_are_detected() -> TestResult {
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen("  \"codexy-watcher.toml\",\n", "", 1)
    })?;
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen(
            "  \"codexy-watcher.toml\",\n]",
            "  \"codexy-watcher.toml\",\n  \"codexy-forge.toml\",\n]",
            1,
        )
    })?;
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen(
            "  \"codexy-watcher.toml\",\n]",
            "  \"codexy-watcher.toml\",\n  \"codexy-watcher.toml\",\n]",
            1,
        )
    })?;
    Ok(())
}

fn assert_current_catalog(plugin_root: &Path) -> TestResult {
    let entries = read_catalog_entries(plugin_root)?;
    let configured = entries.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        configured,
        expected_catalog_files(),
        "current specialist catalog membership must be exact"
    );
    assert_eq!(
        entries.len(),
        configured.len(),
        "current specialist catalog must not contain duplicate entries"
    );
    Ok(())
}

fn assert_retired_specialists(plugin_root: &Path) -> TestResult {
    let configured = read_catalog_entries(plugin_root)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    for name in RETIRED {
        let filename = format!("{name}.toml");
        assert!(
            !configured.contains(&filename),
            "retired specialist remains in the packaged catalog: {name}"
        );
        let output = resolve_named_specialist(plugin_root, name)?;
        assert!(
            !output.status.success(),
            "resolver accepted retired specialist: {name}"
        );
    }
    Ok(())
}

fn assert_catalog_mutation_is_detected(
    mutate: impl FnOnce(String) -> String,
) -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let catalog_path = plugin_root.join("agents/catalog.toml");
    let catalog = std::fs::read_to_string(&catalog_path)?;
    let mutated = mutate(catalog);
    std::fs::write(catalog_path, mutated)?;

    let entries = read_catalog_entries(&plugin_root)?;
    assert!(
        !catalog_membership_is_valid(&entries),
        "disposable catalog mutation was not detected"
    );
    Ok(())
}

fn catalog_membership_is_valid(entries: &[String]) -> bool {
    let configured = entries.iter().cloned().collect::<BTreeSet<_>>();
    configured == expected_catalog_files() && entries.len() == configured.len()
}

fn expected_catalog_files() -> BTreeSet<String> {
    CURRENT
        .iter()
        .map(|name| format!("{name}.toml"))
        .collect()
}

fn read_catalog_entries(plugin_root: &Path) -> TestResult<Vec<String>> {
    let catalog = std::fs::read_to_string(plugin_root.join("agents/catalog.toml"))?;
    Ok(toml::from_str::<toml::Value>(&catalog)?["agent_files"]
        .as_array()
        .ok_or("catalog agent_files")?
        .iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or("catalog filename"))
        .collect::<Result<Vec<_>, _>>()?)
}

fn resolve_named_specialist(
    plugin_root: &Path,
    name: &str,
) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(
        &request_path,
        serde_json::to_vec(&json!({
            "schema": "codexy.child-routing-request.v1",
            "classification": "general",
            "named_specialist": name,
            "codex_thread_operation": "create_thread"
        }))?,
    )?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--resolve-child-routing",
            "--routing-request-file",
        ])
        .arg(request_path)
        .output()?)
}
