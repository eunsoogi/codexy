use super::state::{DEFAULT_WAIT_MS, MAX_WAIT_MS};
use serde_json::{Value, json};

use crate::mcp::ToolDef;

const MAX_INPUT_STRING_BYTES: usize = 128;
const MAX_EVIDENCE_ITEMS: usize = 16;
const MAX_EVIDENCE_BYTES: usize = 512;
const CONTROL_FREE_TEXT_PATTERN: &str = r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]+$";
const CONTROL_FREE_VALUE_PATTERN: &str = r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]*$";
const SAFE_ID_PATTERN: &str = r"^(?![\s\S]*[/\\\u0000-\u001F\u007F-\u009F])[\s\S]+$";
const EVENT_KINDS: &[&str] = &[
    "terminal",
    "failure",
    "drift",
    "missing_delivery",
    "gate_ready",
    "unavailable",
];
const WATCHER_STATES: &[&str] = &[
    "starting",
    "running",
    "idle",
    "error",
    "stopped",
    "unavailable",
];

// Reuse field schemas for nested and legacy flat reports so the two wire forms cannot drift.
fn input_string_schema(label: &str) -> Value {
    // JSON Schema counts characters, while the parser's contract is a UTF-8 byte cap.
    json!({
        "type":"string",
        "minLength":1,
        "maxLength":MAX_INPUT_STRING_BYTES,
        "pattern":CONTROL_FREE_TEXT_PATTERN,
        "description":format!("Nonempty {label}, at most {MAX_INPUT_STRING_BYTES} UTF-8 bytes without control characters."),
    })
}

// Assignments and event keys use the runtime's path-free safe_id validation.
fn safe_id_schema(label: &str) -> Value {
    json!({
        "type":"string",
        "minLength":1,
        "maxLength":MAX_INPUT_STRING_BYTES,
        "pattern":SAFE_ID_PATTERN,
        "description":format!("Nonempty {label}, at most {MAX_INPUT_STRING_BYTES} UTF-8 bytes without control characters, slash, or backslash."),
    })
}

fn identity_schema() -> Value {
    let identity_fields = ["id", "threadId", "taskId", "hostId", "kind", "name"];
    // Runtime identity objects require an id or threadId but permit empty optional values.
    let properties = identity_fields
        .iter()
        .map(|field| {
            (
                (*field).to_owned(),
                json!({
                    "type":"string",
                    "maxLength":MAX_INPUT_STRING_BYTES,
                    "pattern":CONTROL_FREE_VALUE_PATTERN,
                    "description":format!("At most {MAX_INPUT_STRING_BYTES} UTF-8 bytes without control characters."),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    json!({
        "oneOf":[
            {
                "type":"string",
                "minLength":1,
                "maxLength":MAX_INPUT_STRING_BYTES,
                "pattern":CONTROL_FREE_TEXT_PATTERN,
                "description":format!("Nonempty identity, at most {MAX_INPUT_STRING_BYTES} UTF-8 bytes without control characters."),
            },
            {
                "type":"object",
                "additionalProperties":false,
                "properties":properties,
                "anyOf":[{"required":["id"]},{"required":["threadId"]}],
            }
        ]
    })
}

fn evidence_schema() -> Value {
    json!({
        "type":"array",
        "maxItems":MAX_EVIDENCE_ITEMS,
        "items":{
            "type":"string",
            "maxLength":MAX_EVIDENCE_BYTES,
            "pattern":CONTROL_FREE_VALUE_PATTERN,
            "description":format!("At most {MAX_EVIDENCE_BYTES} UTF-8 bytes without control characters."),
        }
    })
}

fn event_properties() -> serde_json::Map<String, Value> {
    let mut properties = serde_json::Map::new();
    properties.insert("target".to_owned(), identity_schema());
    properties.insert("eventId".to_owned(), safe_id_schema("event key"));
    properties.insert(
        "kind".to_owned(),
        json!({"type":"string","enum":EVENT_KINDS}),
    );
    properties.insert("summary".to_owned(), input_string_schema("summary"));
    properties.insert("evidence".to_owned(), evidence_schema());
    properties.insert(
        "observedAtMs".to_owned(),
        json!({"type":"integer","minimum":0,"maximum":u64::MAX}),
    );
    properties.insert(
        "watcherState".to_owned(),
        json!({"type":"string","enum":WATCHER_STATES}),
    );
    properties.insert("lastError".to_owned(), input_string_schema("lastError"));
    properties
}

// Keep the advertised wait default and ceiling tied to the state store's accepted limits.
fn wait_tool(name: &str, description: &str) -> ToolDef {
    ToolDef::new(
        name,
        description,
        json!({
            "type":"object", "additionalProperties":false,
            "properties":{"sessionId":{"type":"string"},"parentToken":{"type":"string"},"cursor":{"type":["string","integer"]},"maxReports":{"type":"integer","minimum":1,"maximum":8},"timeoutMs":{"type":"integer","minimum":0,"maximum":MAX_WAIT_MS,"default":DEFAULT_WAIT_MS},"requestBinding":{"type":"string","maxLength":128}},
            "required":["sessionId","parentToken"]
        }),
    )
}

pub fn tools() -> Vec<ToolDef> {
    let report_properties = event_properties();
    let mut report = serde_json::Map::new();
    report.insert("sessionId".to_owned(), input_string_schema("sessionId"));
    report.insert(
        "watcherToken".to_owned(),
        input_string_schema("watcherToken"),
    );
    for (name, schema) in &report_properties {
        report.insert(name.clone(), schema.clone());
    }
    report.insert(
        "event".to_owned(),
        json!({
            "type":"object",
            "additionalProperties":false,
            "properties":report_properties,
            "description":"Optional nested report fields; the legacy top-level event fields remain accepted.",
        }),
    );

    let identity = identity_schema();
    let mut open_properties = serde_json::Map::new();
    open_properties.insert("assignmentId".to_owned(), safe_id_schema("assignment ID"));
    open_properties.insert("parent".to_owned(), identity.clone());
    open_properties.insert("parentId".to_owned(), input_string_schema("parentId"));
    open_properties.insert("watcher".to_owned(), identity.clone());
    open_properties.insert("watcherId".to_owned(), input_string_schema("watcherId"));
    open_properties.insert(
        "targets".to_owned(),
        json!({
            "type":"array",
            "minItems":1,
            "maxItems":8,
            "items":identity,
        }),
    );
    open_properties.insert(
        "ttlSeconds".to_owned(),
        json!({"type":"integer","minimum":1,"maximum":86400}),
    );

    vec![
        ToolDef::new(
            "watcher_open",
            "Open one bounded Watcher transport session; observation and judgement stay outside MCP.",
            json!({
                "type":"object",
                "additionalProperties":false,
                "properties":open_properties,
                "required":["assignmentId","targets"],
                "allOf":[{"anyOf":[{"required":["parent"]},{"required":["parentId"]}]},{"anyOf":[{"required":["watcher"]},{"required":["watcherId"]}]}]
            }),
        ),
        ToolDef::new(
            "watcher_report",
            "Publish a bounded native Watcher observation or material event; reports are untrusted signals.",
            json!({
                "type":"object",
                "additionalProperties":false,
                "properties":report,
                "required":["sessionId","watcherToken"],
                "anyOf":[
                    {"required":["target"]},
                    {"required":["event"],"properties":{"event":{"required":["target"]}}}
                ]
            }),
        ),
        wait_tool(
            "watcher_wait",
            "Wait for bounded material Watcher reports from a durable cross-process queue for up to 60 minutes. Omitting timeoutMs selects the 295-second server-side default of 295000 ms, leaving a five-second margin under the observed 300-second tools/call transport deadline; an explicit timeoutMs=295000 is equivalent. The separate MAX_WAIT_MS maximum remains 3600000 ms, and shorter waits require a stated reason such as a user deadline, confirmed host limit, or diagnostic purpose. The optional requestBinding is injected by the Codex PreToolUse hook and is validated against the authenticated parent capability; direct callers may omit it. A same-connection MCP notifications/cancelled request or supported host Interrupt releases only this request and preserves the durable session. watcher_cancel is separate: it durably ends the session and requires a fresh assignment for observation. If the session TTL expires, returns status=expired with empty events and the unchanged nextCursor; the wait does not consume or modify the durable event log.",
        ),
        ToolDef::new(
            "watcher_health",
            "Return transport and native-observation freshness metadata; health is not semantic acceptance proof.",
            json!({
                "type":"object", "additionalProperties":false,
                "properties":{"sessionId":{"type":"string"},"token":{"type":"string"},"parentToken":{"type":"string"},"watcherToken":{"type":"string"}},
                "required":["sessionId","token"]
            }),
        ),
        ToolDef::new(
            "watcher_cancel",
            "Durably cancel a Watcher session and wake an independent waiter.",
            json!({
                "type":"object", "additionalProperties":false,
                "properties":{"sessionId":{"type":"string"},"parentToken":{"type":"string"}},
                "required":["sessionId","parentToken"]
            }),
        ),
    ]
}
