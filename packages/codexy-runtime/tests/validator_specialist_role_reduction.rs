// Keep the packaged specialist set exact: retired names stay uncallable even
// when historical prose mentions them, and catalog drift is rejected.
use std::{collections::BTreeSet, path::Path, process::Command};

use serde_json::json;

use crate::support::{TestResult, copy_plugin_fixture_with_mutable_files};

const CURRENT: [&str; 9] = [
    "codexy-architect",
    "codexy-advisor",
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
fn advisor_remains_resolvable_as_a_named_specialist() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let output = resolve_named_specialist(&plugin_root, "codexy-advisor")?;
    assert!(
        output.status.success(),
        "new Advisor role must remain callable through named specialist routing"
    );
    Ok(())
}

#[test]
fn recipient_follow_ups_require_assignment_and_advertised_capabilities() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let cases = [
        (
            json!({
                "schema":"codexy.child-routing-request.v2",
                "classification":"general",
                "codex_thread_operation":"send_message_to_thread",
                "codex_thread_direction":"parent_to_generic",
                "codex_thread_capabilities":{"models":[{"model":"gpt-6-luna","thinking":["max"]}]}
            }),
            "Recipient-bound follow-up routing requires recipient_assignment in schema v2; sender settings are not a substitute.",
        ),
        (
            json!({
                "schema":"codexy.child-routing-request.v2",
                "classification":"general",
                "codex_thread_operation":"send_message_to_thread",
                "codex_thread_direction":"parent_to_generic",
                "recipient_assignment":{"model":"gpt-6.1-sol","thinking":"medium"},
                "codex_thread_capabilities":{"models":[{"model":"gpt-6-luna","thinking":["max"]}]}
            }),
            "Assigned recipient model and thinking are not supported by declared thread capabilities.",
        ),
    ];
    for (request, reason) in cases {
        let output = codexy_runtime::validation::resolve_child_routing(
            &plugin_root,
            &serde_json::to_string(&request)?,
        )?;
        assert_eq!(output, json!({"route":"root_or_named_specialist","reason":reason}));
    }

    let incomplete = json!({
        "schema":"codexy.child-routing-request.v2",
        "classification":"general",
        "codex_thread_operation":"send_message_to_thread",
        "codex_thread_direction":"parent_to_generic",
        "recipient_assignment":{"model":"gpt-6.1-sol"}
    });
    let error = codexy_runtime::validation::resolve_child_routing(
        &plugin_root,
        &serde_json::to_string(&incomplete)?,
    )
    .expect_err("partial recipient assignment must not route");
    assert!(error.to_string().contains("missing field `thinking`"));
    Ok(())
}

#[test]
fn retired_specialists_are_not_registered_or_callable() -> TestResult {
    let plugin_root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    assert_retired_specialists(&plugin_root)
}

#[test]
fn historical_prose_does_not_define_current_role_contract() -> TestResult {
    let mutable_files = [
        Path::new("skills/orchestration/SKILL.md"),
        Path::new("skills/orchestration/references/classification-and-control.md"),
    ];
    let (temp, plugin_root) = copy_plugin_fixture_with_mutable_files(&mutable_files)?;
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
    let mutable_files = [Path::new("agents/catalog.toml")];
    let (_temp, plugin_root) = copy_plugin_fixture_with_mutable_files(&mutable_files)?;
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
