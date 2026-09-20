use serde_json::{Value, json};

pub(super) fn change_impact_schema() -> Value {
    analysis_schema(false)
}

pub(super) fn check_selection_schema() -> Value {
    let mut schema = analysis_schema(true);
    schema["properties"]["dependencyState"] = json!({
        "type": "string",
        "enum": ["confirmed", "unconfirmed"],
        "default": "unconfirmed"
    });
    schema["properties"]["dependencyDetail"] = json!({"type": "string"});
    schema["required"] = json!(["mappings"]);
    schema
}

fn analysis_schema(with_mappings: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "root": {"type": "string", "description": "Git worktree root; defaults to the server working directory."},
            "base": {"type": "string", "description": "Git baseline revision for a commit comparison."},
            "head": {"type": "string", "description": "Git head revision for a commit comparison."},
            "includeUntracked": {"type": "boolean", "default": true},
            "maxFiles": {"type": "number", "minimum": 1, "description": "Maximum files inspected by impact analysis."},
            "maxPaths": {"type": "number", "minimum": 1, "description": "Maximum causal paths retained by impact analysis."}
        }
    });
    if with_mappings {
        schema["properties"]["mappings"] = json!({
            "type": "object",
            "description": "Explicit check definitions and path/configuration/fixture mappings.",
            "properties": {
                "checks": {"type": "array", "items": {"type": "object", "properties": {"id": {"type": "string"}, "command": {"type": "string"}, "description": {"type": "string"}}, "required": ["id", "command", "description"]}},
                "mappings": {"type": "array", "items": {"type": "object", "properties": {"owner": {"type": "string", "enum": ["user", "repository"]}, "kind": {"type": "string", "enum": ["path", "shared_configuration", "fixture"]}, "pattern": {"type": "string"}, "checkIds": {"type": "array", "items": {"type": "string"}}, "reason": {"type": "string"}}, "required": ["owner", "kind", "pattern", "checkIds", "reason"]}}
            },
            "required": ["checks", "mappings"]
        });
    }
    schema
}
