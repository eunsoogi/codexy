use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{
    Policy,
    routes::{bounded_evidence_is_complete, selected_general_route, simple_route},
    thread_capabilities::{self, Request},
};

/// Resolves a creation route, then attaches the same complete input the app tool accepts.
pub(super) fn resolve(policy: &Policy, request: &Request) -> Result<Value> {
    let route = match request.classification.as_str() {
        "bounded_evidence" if bounded_evidence_is_complete(request.simple_predicates.as_ref()) => {
            Ok(simple_route(
                policy,
                request.codex_thread_capabilities.as_ref(),
                &request.codex_thread_operation,
            ))
        }
        "simple" if thread_capabilities::simple_is_complete(request.simple_predicates.as_ref()) => {
            Ok(simple_route(
                policy,
                request.codex_thread_capabilities.as_ref(),
                &request.codex_thread_operation,
            ))
        }
        "general"
            if thread_capabilities::worker_scope_is_complete(
                request.simple_predicates.as_ref(),
            ) =>
        {
            Ok(selected_general_route(
                policy,
                request.codex_thread_capabilities.as_ref(),
                &request.codex_thread_operation,
                request.hard_work.is_some(),
            ))
        }
        "general" | "simple" | "bounded_evidence" | "ambiguous" | "high_risk" | "incomplete" => {
            Ok(json!({"route":policy.fallback}))
        }
        _ => bail!("Worker routing request classification is not recognized"),
    };
    route.map(|route| {
        attach_tool_input(
            route,
            request.creation_prompt.as_deref(),
            request.creation_target.as_ref(),
            &policy.fallback,
        )
    })
}

/// Builds the app arguments for a selected Worker route without relying on host defaults.
fn attach_tool_input(
    mut route: Value,
    prompt: Option<&str>,
    target: Option<&Value>,
    fallback_route: &str,
) -> Value {
    if route.get("route").and_then(Value::as_str) != Some("generic") {
        return route;
    }

    let Some(prompt) = prompt.filter(|prompt| !prompt.trim().is_empty()) else {
        return fallback(
            fallback_route,
            "Worker creation requires a non-empty creation_prompt.",
        );
    };
    let Some(target) = target.filter(|target| is_project_worktree_target(target)) else {
        return fallback(
            fallback_route,
            "Worker creation requires a project target with a non-empty projectId without surrounding whitespace and a worktree environment.",
        );
    };
    let (Some(model), Some(thinking)) = (
        route
            .get("model")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        route
            .get("thinking")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    ) else {
        return fallback(
            fallback_route,
            "Worker creation route is missing its selected model or thinking value.",
        );
    };
    let Some(route_fields) = route.as_object_mut() else {
        return fallback(
            fallback_route,
            "Worker creation route is not a request object.",
        );
    };

    route_fields.insert(
        "tool_input".to_owned(),
        json!({
            "prompt":prompt,
            "model":model,
            "thinking":thinking,
            "target":target
        }),
    );
    route
}

// Match admission-hook validation without silently normalizing the owner's target.
fn is_project_worktree_target(target: &Value) -> bool {
    target.get("type").and_then(Value::as_str) == Some("project")
        && target
            .get("projectId")
            .and_then(Value::as_str)
            .is_some_and(|project_id| !project_id.is_empty() && project_id == project_id.trim())
        && target
            .get("environment")
            .and_then(|environment| environment.get("type"))
            .and_then(Value::as_str)
            == Some("worktree")
}

fn fallback(route: &str, reason: &str) -> Value {
    json!({"route":route,"reason":reason})
}
