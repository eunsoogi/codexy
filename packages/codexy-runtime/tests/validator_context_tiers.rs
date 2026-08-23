use std::path::Path;

use serde_json::{Value, json};

use super::{
    assert_context_contract_rejected as assert_rejected, check_context_contract as check,
    CONTEXT_CONTRACT as CONTRACT, CONTEXT_TASK_CLASSES as TASK_CLASSES,
};
use crate::support::{self, TestResult};

const PROFILES: [&str; 3] = ["light", "standard", "strict"];
const CLASSIFICATION_REFERENCES: [&str; 2] = ["workflow_profiles", "tdd_classification_policy"];

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
fn profile_tier_behavior_stable_volatile_identities_and_public_routes_are_separate() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
    let authorities = contract["authorities"].as_array().ok_or("authorities")?;
    assert!(authorities.iter().any(|authority| {
        authority["id"] == "tdd_classification_policy"
            && authority["path"] == "skills/orchestration/references/tdd-classification-policy.json"
    }));
    assert!(authorities.iter().any(|authority| {
        authority["id"] == "public_extension_contracts"
            && authority["path"] == "skills/orchestration/references/plugin-public-contracts.md"
    }));
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
    let baseline = identities(&root, &current)?;
    let mut stable_change = current.clone();
    stable_change["slots"]["selected_references"] = json!({"value":["other"]});
    let changed = identities(&root, &stable_change)?;
    assert_ne!(baseline[0], changed[0]);
    assert_eq!(baseline[1], changed[1]);
    for task_class in TASK_CLASSES {
        assert_task_route(&root, &current, &contract, task_class)?;
    }
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
        assert!(rejected(&root, &baseline_envelope, &invalid)?, "{field}");
    }
    let mut ambiguous = current.clone();
    ambiguous["slots"]["checks"] = json!({"value":"pending","extra":true});
    assert!(identities(&root, &ambiguous).is_err());

    for (item, should_reject) in [
        ("full_tool_payload", true),
        ("raw conversation text", true),
        ("full_conversation_forwarding", true),
        ("retained_slot", false),
        ("selected_reference", false),
        ("qualifying_event_delta", false),
        ("authoritative_refresh_handle", false),
    ] {
        let mut forwarded = baseline_envelope.clone();
        forwarded["forwarded_context"] = json!([item]);
        assert_eq!(rejected(&root, &forwarded, &current)?, should_reject, "{item}");
    }

    for mutate in [
        |value: &mut Value| value["schema"] = json!("other"),
        |value: &mut Value| {
            value["slots"].as_object_mut().expect("slots").remove("checks");
        },
        |value: &mut Value| value["slots"]["checks"] = json!({"value":{"raw":"pending"}}),
    ] {
        let mut invalid = current.clone();
        mutate(&mut invalid);
        assert!(identities(&root, &invalid).is_err());
    }

    let mut omitted_thread = baseline_envelope.clone();
    omitted_thread["slots"]["unresolved_review_threads"] =
        json!({"omitted":{"code":"not_applicable","reason":"none retained"}});
    assert!(!diagnostics(&root, &omitted_thread, &current)?.is_empty());

    for risk in ["unknown", "ambiguous", "high_risk", "security", "permission", "release"] {
        let mut risk_current = current.clone();
        risk_current["slots"]["task_classification"] = json!({"value":risk});
        let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
        risk_current["slots"]["selected_references"] =
            json!({"value":contract["routing"]["fallback_reference_route"].clone()});
        let mut routed = envelope(&root, &risk_current)?;
        routed["task_class"] = json!(risk);
        routed["route_authority"] = json!("child_routing");
        routed["action_allowed"] = json!(false);
        assert!(diagnostics(&root, &routed, &risk_current)?.is_empty(), "{risk}");
        routed["action_allowed"] = json!(true);
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

fn identities(root: &Path, state: &Value) -> TestResult<[String; 2]> {
    Ok(codexy_runtime::validation::context_identities(
        root,
        &serde_json::to_string(state)?,
    )?)
}

fn rejected(root: &Path, envelope: &Value, current: &Value) -> TestResult<bool> {
    Ok(diagnostics(root, envelope, current).map_or(true, |errors| !errors.is_empty()))
}

fn assert_task_route(root: &Path, current: &Value, contract: &Value, task_class: &str) -> TestResult {
    let mut routed = current.clone();
    routed["slots"]["task_classification"] = json!({"value":task_class});
    let route = super::validator_task_surface_routing::contract_route(contract, task_class)?;
    let references = route["value"].as_array().ok_or("route authorities")?;
    assert!(CLASSIFICATION_REFERENCES.iter().all(|required| {
        references.iter().any(|authority| authority == required)
    }));
    assert_eq!(
        references.iter().any(|authority| authority == "public_extension_contracts"),
        matches!(task_class, "orchestration/lane setup" | "review response" | "GitHub/merge" | "plugin/release" | "issue/intake only"),
        "{task_class}"
    );
    routed["slots"]["selected_references"] = route;
    let retained = envelope(root, &routed)?;
    assert!(diagnostics(root, &retained, &routed)?.is_empty(), "{task_class}");
    Ok(())
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
            "selected_references":{"value":["workflow_profiles","task_classification","tdd_classification_policy","execution_budget","proof_completion"]},
            "qualifying_event_delta":{"value":"new-head"},
            "authoritative_refresh_handles":{"value":["git","github"]}
        }
    })
}

fn envelope(root: &Path, current: &Value) -> TestResult<Value> {
    let identities = identities(root, current)?;
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
