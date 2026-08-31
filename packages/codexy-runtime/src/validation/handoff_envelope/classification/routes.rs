const TASK_CLASSES: [&str; 10] = [
    "orchestration/lane setup",
    "implementation",
    "review response",
    "GitHub/merge",
    "validation/QA",
    "documentation/skill authoring",
    "plugin/release",
    "investigation/debugging",
    "issue/intake only",
    "other",
];
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
const FAIL_CLOSED_CLASSES: [&str; 6] = [
    "unknown",
    "ambiguous",
    "high_risk",
    "security",
    "permission",
    "release",
];

pub(super) fn known_workflow(value: &str) -> bool {
    TASK_CLASSES.contains(&value) || FAIL_CLOSED_CLASSES.contains(&value)
}

pub(super) fn known_surface(value: &String) -> bool {
    SURFACES.contains(&value.as_str())
}

pub(super) fn known_risk(value: &String) -> bool {
    RISKS.contains(&value.as_str())
}

pub(super) fn fail_closed_class(value: &str) -> bool {
    FAIL_CLOSED_CLASSES.contains(&value)
}

pub(super) fn fallback_route() -> Vec<String> {
    owned(&["workflow_profiles", "task_classification", "child_routing"])
}

pub(super) fn task_route(workflow: &str) -> Option<Vec<String>> {
    Some(owned(match workflow {
        "orchestration/lane setup" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "child_routing",
            "execution_budget",
            "public_extension_contracts",
        ],
        "implementation" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "execution_budget",
            "proof_completion",
        ],
        "review response" | "GitHub/merge" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "review_profiles",
            "review_lifecycle",
            "proof_completion",
            "public_extension_contracts",
        ],
        "validation/QA" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "execution_budget",
            "review_profiles",
            "proof_completion",
        ],
        "documentation/skill authoring" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "proof_completion",
        ],
        "plugin/release" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "execution_budget",
            "proof_completion",
            "public_extension_contracts",
        ],
        "investigation/debugging" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "execution_budget",
            "proof_completion",
        ],
        "issue/intake only" => &[
            "workflow_profiles",
            "task_classification",
            "tdd_classification_policy",
            "child_routing",
            "public_extension_contracts",
        ],
        "other" => &[
            "workflow_profiles",
            "tdd_classification_policy",
            "child_routing",
        ],
        _ => return None,
    }))
}

pub(super) fn surface_route(surface: &str) -> Option<Vec<String>> {
    Some(owned(match surface {
        "repository engineering" => &["proof_completion"],
        "GitHub" => &[
            "review_profiles",
            "review_lifecycle",
            "proof_completion",
            "public_extension_contracts",
        ],
        "browser/desktop" | "documents/artifacts" | "spreadsheets/data" => &[
            "workflow_profiles",
            "task_classification",
            "proof_completion",
        ],
        "research/wiki" => &["dreaming"],
        "read-only/local" => &["task_classification"],
        _ => return None,
    }))
}

pub(super) fn risk_route(risk: &str) -> Option<Vec<String>> {
    Some(owned(match risk {
        "mixed" => &["workflow_profiles", "task_classification", "child_routing"],
        "security" | "permission" | "destructive" => &[
            "workflow_profiles",
            "task_classification",
            "child_routing",
            "proof_completion",
        ],
        "external_mutation" => &[
            "child_routing",
            "proof_completion",
            "public_extension_contracts",
        ],
        _ => return None,
    }))
}

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}
