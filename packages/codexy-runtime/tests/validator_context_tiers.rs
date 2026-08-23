use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

const CONTRACT: &str = "skills/orchestration/references/context-tiers.json";
const TIERS: [&str; 4] = ["always_on", "task_selected", "event_delta", "refresh_only"];
const PROFILES: [&str; 3] = ["light", "standard", "strict"];
const SAFETY_FIELDS: [&str; 10] = [
    "issue_pr_identity",
    "owner_worktree",
    "base_head_sha",
    "dirty_index_state",
    "checks",
    "unresolved_review_threads",
    "selected_reviewer_state",
    "verification",
    "external_gate",
    "next_action",
];

#[test]
fn contract_covers_every_tier_profile_and_safety_invariant() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy").join(CONTRACT),
    )?)?;

    assert_eq!(contract["schema"], "codexy.context-tiers.v1");
    assert_eq!(contract["tier_order"], json!(TIERS));
    for profile in PROFILES {
        let tiers = contract["profile_matrix"][profile]
            .as_object()
            .ok_or("profile tier matrix")?;
        assert_eq!(tiers.len(), TIERS.len(), "{profile} must decide every tier");
        assert!(TIERS.iter().all(|tier| tiers.contains_key(*tier)));
    }
    let fields = contract["retained_fields"]
        .as_array()
        .ok_or("retained fields")?;
    for required in SAFETY_FIELDS {
        let field = fields
            .iter()
            .find(|field| field["name"] == required)
            .ok_or_else(|| format!("missing safety field {required}"))?;
        assert_eq!(field["safety_invariant"], true);
        assert_eq!(field["budget_exempt"], true);
    }
    assert_eq!(contract["budget_semantics"]["cache_metadata"], "unavailable_not_zero");
    Ok(())
}

#[test]
fn validator_rejects_unknown_tiers_duplicate_authority_and_safety_weakening() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(CONTRACT)])?;
    let path = fixture.root().join(CONTRACT);
    let baseline: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert!(check(fixture.root())?.status.success());

    assert_rejected(fixture.root(), &path, &baseline, |value| {
        value["tier_order"][0] = json!("unknown");
    })?;
    assert_rejected(fixture.root(), &path, &baseline, |value| {
        let duplicate = value["authorities"][0].clone();
        value["authorities"].as_array_mut().expect("authorities").push(duplicate);
    })?;
    assert_rejected(fixture.root(), &path, &baseline, |value| {
        value["retained_fields"]
            .as_array_mut()
            .expect("fields")
            .retain(|field| field["name"] != "base_head_sha");
    })?;
    assert_rejected(fixture.root(), &path, &baseline, |value| {
        value["budget_semantics"]["cache_metadata"] = json!("infer_zero");
    })?;
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        &path,
        text.replacen(
            "\"schema\":\"codexy.context-tiers.v1\"",
            "\"schema\":\"other\",\"schema\":\"codexy.context-tiers.v1\"",
            1,
        ),
    )?;
    assert!(!check(fixture.root())?.status.success(), "duplicate JSON key passed");
    Ok(())
}

#[test]
fn profile_tier_behavior_and_stable_volatile_identities_are_separate() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    for profile in PROFILES {
        for (field, action_allowed, permitted) in [
            ("issue_pr_identity", true, false),
            ("selected_references", true, false),
            ("qualifying_event_delta", true, true),
            ("authoritative_refresh_handles", true, profile == "light"),
            ("authoritative_refresh_handles", false, true),
        ] {
            let mut current = current_state();
            current["slots"]["workflow_profile"] = json!({"value":profile});
            current["slots"][field] =
                json!({"omitted":{"code":"not_applicable","reason":"matrix case"}});
            let mut retained = envelope(&root, &current)?;
            retained["action_allowed"] = json!(action_allowed);
            assert_eq!(diagnostics(&root, &retained, &current)?.is_empty(), permitted);
        }
    }
    let current = current_state();
    let baseline = codexy_runtime::validation::context_identities(
        &root,
        &serde_json::to_string(&current)?,
    )?;
    let mut stable_change = current.clone();
    stable_change["slots"]["selected_references"] = json!({"value":["other"]});
    let changed = codexy_runtime::validation::context_identities(
        &root,
        &serde_json::to_string(&stable_change)?,
    )?;
    assert_ne!(baseline[0], changed[0]);
    assert_eq!(baseline[1], changed[1]);
    Ok(())
}

#[test]
fn envelope_rejects_stale_or_missing_current_safety_state_and_forbidden_forwarding() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let current = current_state();
    let baseline_envelope = envelope(&root, &current)?;
    assert!(diagnostics(&root, &baseline_envelope, &current)?.is_empty());

    let mut stale = baseline_envelope.clone();
    stale["slots"]["base_head_sha"] = json!({"value":{"base":"base","head":"stale"}});
    assert!(!diagnostics(&root, &stale, &current)?.is_empty());

    let mut missing_owner = baseline_envelope.clone();
    missing_owner["slots"].as_object_mut().expect("slots").remove("owner_worktree");
    assert!(!diagnostics(&root, &missing_owner, &current)?.is_empty());

    for (field, value) in [
        ("base_head_sha", json!({"value":{"base":"base"}})),
        ("selected_references", json!({"value":["task_classification"]})),
        ("selected_references", json!({"value":{"body":"full conversation"}})),
        ("qualifying_event_delta", json!({"value":"tool payload body"})),
        ("authoritative_refresh_handles", json!({"value":["document body"]})),
    ] {
        let mut invalid = current.clone();
        invalid["slots"][field] = value;
        let retained = envelope(&root, &invalid)?;
        assert!(!diagnostics(&root, &retained, &invalid)?.is_empty(), "{field}");
    }
    let mut ambiguous = current.clone();
    ambiguous["slots"]["checks"] = json!({"value":"pending","extra":true});
    assert!(codexy_runtime::validation::context_identities(
        &root,
        &serde_json::to_string(&ambiguous)?,
    )
    .is_err());

    let mut omitted_thread = baseline_envelope.clone();
    omitted_thread["slots"]["unresolved_review_threads"] =
        json!({"omitted":{"code":"not_applicable","reason":"none retained"}});
    assert!(!diagnostics(&root, &omitted_thread, &current)?.is_empty());

    let mut forwarded = baseline_envelope;
    forwarded["forwarded_context"] = json!(["full_conversation_forwarding"]);
    assert!(!diagnostics(&root, &forwarded, &current)?.is_empty());

    for risk in ["unknown", "ambiguous", "high_risk", "security", "permission", "release"] {
        let mut risk_current = current.clone();
        risk_current["slots"]["task_classification"] = json!({"value":risk});
        let mut routed = envelope(&root, &risk_current)?;
        routed["task_class"] = json!(risk);
        routed["route_authority"] = json!("child_routing");
        assert!(diagnostics(&root, &routed, &risk_current)?.is_empty(), "{risk}");
        routed["route_authority"] = Value::Null;
        assert!(!diagnostics(&root, &routed, &risk_current)?.is_empty());
    }
    Ok(())
}

fn diagnostics(root: &Path, envelope: &Value, current: &Value) -> TestResult<Vec<String>> {
    Ok(codexy_runtime::validation::validate_context_envelope(
        root,
        &serde_json::to_string(envelope)?,
        &serde_json::to_string(current)?,
    )?)
}

fn current_state() -> Value {
    json!({
        "schema":"codexy.context-current-state.v1",
        "slots": {
            "workflow_profile":{"value":"strict"},
            "task_classification":{"value":"implementation"},
            "issue_pr_identity":{"value":{"issue":598,"pr":null}},
            "owner_worktree":{"value":{"owner":"child-owned","worktree":"reserved"}},
            "base_head_sha":{"value":{"base":"base","head":"head"}},
            "dirty_index_state":{"value":{"dirty":false,"index":false}},
            "checks":{"value":"not_created"},
            "unresolved_review_threads":{"value":["thread-1"]},
            "selected_reviewer_state":{"value":"pending"},
            "verification":{"value":["focused"]},
            "external_gate":{"value":"none"},
            "next_action":{"value":"implement"},
            "selected_references":{"value":["task_classification","execution_budget","proof_completion"]},
            "qualifying_event_delta":{"value":"new-head"},
            "authoritative_refresh_handles":{"value":["git","github"]}
        }
    })
}

fn envelope(root: &Path, current: &Value) -> TestResult<Value> {
    let current_text = serde_json::to_string(current)?;
    let identities = codexy_runtime::validation::context_identities(root, &current_text)?;
    Ok(json!({
        "schema":"codexy.context-envelope.v1",
        "profile":current["slots"]["workflow_profile"]["value"].clone(),
        "task_class":current["slots"]["task_classification"]["value"].clone(),
        "route_authority":null,
        "action_allowed":true,
        "slots":current["slots"].clone(),
        "forwarded_context":[],
        "stable_identity":identities[0],
        "volatile_identity":identities[1]
    }))
}

fn assert_rejected(
    root: &Path,
    path: &Path,
    baseline: &Value,
    mutate: impl FnOnce(&mut Value),
) -> TestResult {
    let mut invalid = baseline.clone();
    mutate(&mut invalid);
    std::fs::write(path, serde_json::to_vec(&invalid)?)?;
    let output = check(root)?;
    assert!(!output.status.success(), "invalid context contract passed");
    std::fs::write(path, serde_json::to_vec(baseline)?)?;
    Ok(())
}

fn check(root: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--check"])
        .output()?)
}
