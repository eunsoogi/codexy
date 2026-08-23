use serde_json::{Value, json};

use crate::support::TestResult;

const CONTRACT: &str = "skills/orchestration/references/context-tiers.json";
const SURFACES: [&str; 7] = [
    "repository engineering",
    "GitHub",
    "browser/desktop",
    "documents/artifacts",
    "spreadsheets/data",
    "research/wiki",
    "read-only/local",
];
const RISKS: [&str; 5] = [
    "mixed",
    "security",
    "permission",
    "destructive",
    "external_mutation",
];

#[test]
fn context_contract_declares_closed_surface_and_risk_routes() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
    assert_eq!(contract["routing"]["surface_names"], json!(SURFACES));
    assert_eq!(contract["routing"]["risk_names"], json!(RISKS));
    for surface in SURFACES {
        assert!(contract["routing"]["surface_reference_routes"][surface].is_array());
    }
    for risk in RISKS {
        assert!(contract["routing"]["risk_reference_routes"][risk].is_array());
    }
    Ok(())
}

#[test]
fn every_closed_surface_accepts_a_structured_route() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
    for surface in SURFACES {
        let current = surface_state(&contract, surface)?;
        let identities = codexy_runtime::validation::context_identities(
            &root,
            &serde_json::to_string(&current)?,
        )?;
        let envelope = json!({
            "schema":"codexy.context-envelope.v1",
            "profile":"light",
            "task_class":"other",
            "route_authority":null,
            "action_allowed":true,
            "slots":current["slots"].clone(),
            "forwarded_context":[],
            "stable_identity":identities[0],
            "volatile_identity":identities[1]
        });
        let errors = codexy_runtime::validation::validate_context_envelope(
            &root,
            &serde_json::to_string(&envelope)?,
            &serde_json::to_string(&current)?,
        )?;
        assert!(
            errors.is_empty(),
            "structured {surface} route rejected: {errors:?}"
        );
    }
    Ok(())
}

#[test]
fn every_declared_risk_route_fails_closed_without_authority_union() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
    for risk in RISKS {
        let mut risk_state = structured_state();
        risk_state["slots"]["workflow_profile"] = json!({"value":"strict"});
        risk_state["slots"]["task_classification"]["value"]["risks"] = json!([risk]);
        risk_state["slots"]["selected_references"] = risk_route(&contract, risk)?;
        let identities = codexy_runtime::validation::context_identities(
            &root,
            &serde_json::to_string(&risk_state)?,
        )?;
        let envelope = json!({
            "schema":"codexy.context-envelope.v1",
            "profile":"strict",
            "task_class":"other",
            "route_authority":"child_routing",
            "action_allowed":true,
            "slots":risk_state["slots"].clone(),
            "forwarded_context":[],
            "stable_identity":identities[0],
            "volatile_identity":identities[1]
        });
        let mut safe_envelope = envelope.clone();
        safe_envelope["action_allowed"] = json!(false);
        let safe_errors = codexy_runtime::validation::validate_context_envelope(
            &root,
            &serde_json::to_string(&safe_envelope)?,
            &serde_json::to_string(&risk_state)?,
        )?;
        assert!(
            safe_errors.is_empty(),
            "safe {risk} route rejected: {safe_errors:?}"
        );
        let errors = codexy_runtime::validation::validate_context_envelope(
            &root,
            &serde_json::to_string(&envelope)?,
            &serde_json::to_string(&risk_state)?,
        )?;
        assert!(!errors.is_empty(), "unsafe {risk} route was accepted");
    }
    Ok(())
}

#[test]
fn github_actionable_context_must_retain_base_head_sha() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(root.join(CONTRACT))?)?;
    let mut current = surface_state(&contract, "GitHub")?;
    current["slots"]["base_head_sha"] = json!({"omitted":{"code":"not_applicable","reason":"GitHub omission"}});
    let identities = codexy_runtime::validation::context_identities(&root, &serde_json::to_string(&current)?)?;
    let envelope = json!({"schema":"codexy.context-envelope.v1","profile":"light","task_class":"other","route_authority":null,"action_allowed":true,"slots":current["slots"].clone(),"forwarded_context":[],"stable_identity":identities[0],"volatile_identity":identities[1]});
    let errors = codexy_runtime::validation::validate_context_envelope(&root, &serde_json::to_string(&envelope)?, &serde_json::to_string(&current)?)?;
    assert!(!errors.is_empty(), "actionable GitHub route omitted base_head_sha");
    Ok(())
}

fn surface_state(contract: &Value, surface: &str) -> TestResult<Value> {
    let mut state = structured_state();
    state["slots"]["task_classification"]["value"] = json!({
        "workflow":"other",
        "surfaces":[surface],
        "risks":[]
    });
    state["slots"]["selected_references"] = route_for(contract, surface)?;
    let non_applicable = contract["routing"]["surface_non_applicable_fields"][surface]
        .as_array()
        .ok_or("surface applicability")?;
    let slots = state["slots"].as_object_mut().ok_or("state slots")?;
    for field in [
        "issue_pr_identity",
        "owner_worktree",
        "base_head_sha",
        "dirty_index_state",
        "unresolved_review_threads",
        "selected_reviewer_state",
    ] {
        if non_applicable.iter().all(|item| item != field) {
            slots[field] = match field {
                "issue_pr_identity" => json!({"value":{"issue":663,"pr":null}}),
                "owner_worktree" => json!({"value":{"owner":"child-owned","worktree":"reserved"}}),
                "base_head_sha" => json!({"value":{"base":"base","head":"head"}}),
                "dirty_index_state" => json!({"value":{"dirty":false,"index":false}}),
                "unresolved_review_threads" => json!({"value":[]}),
                "selected_reviewer_state" => json!({"value":"pending"}),
                _ => unreachable!("listed field must be handled"),
            };
        }
    }
    Ok(state)
}

fn route_for(contract: &Value, surface: &str) -> TestResult<Value> {
    let mut references = contract["routing"]["task_reference_routes"]["other"]
        .as_array()
        .ok_or("task route authorities")?
        .clone();
    for reference in contract["routing"]["surface_reference_routes"][surface]
        .as_array()
        .ok_or("surface route authorities")?
    {
        if !references.contains(reference) {
            references.push(reference.clone());
        }
    }
    Ok(json!({"value":references}))
}

fn risk_route(contract: &Value, risk: &str) -> TestResult<Value> {
    let mut references = contract["routing"]["fallback_reference_route"]
        .as_array()
        .ok_or("fallback route authorities")?
        .clone();
    for reference in contract["routing"]["risk_reference_routes"][risk]
        .as_array()
        .ok_or("risk route authorities")?
    {
        if !references.contains(reference) {
            references.push(reference.clone());
        }
    }
    Ok(json!({"value":references}))
}

fn structured_state() -> Value {
    json!({
        "schema":"codexy.context-current-state.v1",
        "slots": {
            "workflow_profile":{"value":"light"},
            "task_classification":{"value":{
                "workflow":"other",
                "surfaces":["read-only/local"],
                "risks":[]
            }},
            "issue_pr_identity":{"omitted":{"code":"not_applicable","reason":"no repository surface"}},
            "owner_worktree":{"omitted":{"code":"not_applicable","reason":"no repository surface"}},
            "base_head_sha":{"omitted":{"code":"not_applicable","reason":"no repository surface"}},
            "dirty_index_state":{"omitted":{"code":"not_applicable","reason":"no repository surface"}},
            "checks":{"value":["not_applicable"]},
            "unresolved_review_threads":{"omitted":{"code":"not_applicable","reason":"no review surface"}},
            "selected_reviewer_state":{"omitted":{"code":"not_applicable","reason":"light local read-only"}},
            "verification":{"value":["readback"]},
            "external_gate":{"value":"none"},
            "next_action":{"value":"read"},
            "selected_references":{"value":["workflow_profiles","task_classification","tdd_classification_policy","child_routing"]},
            "qualifying_event_delta":{"omitted":{"code":"not_created","reason":"initial state"}},
            "authoritative_refresh_handles":{"omitted":{"code":"not_applicable","reason":"no external surface"}}
        }
    })
}

pub(crate) fn contract_route(contract: &Value, task_class: &str) -> TestResult<Value> {
    let mut references = contract["routing"]["task_reference_routes"][task_class]
        .as_array()
        .ok_or("task route authorities")?
        .clone();
    for reference in contract["routing"]["surface_reference_routes"]["repository engineering"]
        .as_array()
        .ok_or("repository surface authorities")?
    {
        if !references.contains(reference) {
            references.push(reference.clone());
        }
    }
    Ok(json!({"value":references}))
}
