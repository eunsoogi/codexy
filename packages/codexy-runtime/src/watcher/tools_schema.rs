use super::state::{DEFAULT_WAIT_MS, MAX_WAIT_MS};
use serde_json::json;

use crate::mcp::ToolDef;

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
    vec![
        ToolDef::new(
            "watcher_open",
            "Open one bounded Watcher transport session; observation and judgement stay outside MCP.",
            json!({
                "type":"object", "additionalProperties":false,
                "properties": {"assignmentId":{"type":"string","maxLength":128},"parent":{"type":["string","object"]},"parentId":{"type":"string","maxLength":128},"watcher":{"type":["string","object"]},"watcherId":{"type":"string","maxLength":128},"targets":{"type":"array","minItems":1,"maxItems":8},"ttlSeconds":{"type":"integer","minimum":1,"maximum":86400}},
                "required":["assignmentId","targets"],
                "allOf":[{"anyOf":[{"required":["parent"]},{"required":["parentId"]}]},{"anyOf":[{"required":["watcher"]},{"required":["watcherId"]}]}]
            }),
        ),
        ToolDef::new(
            "watcher_report",
            "Publish a bounded native Watcher observation or material event; reports are untrusted signals.",
            json!({
                "type":"object", "additionalProperties":false,
                "properties": {"sessionId":{"type":"string"},"watcherToken":{"type":"string"},"target":{"type":["string","object"]},"event":{"type":"object","additionalProperties":false,"properties":{"target":{"type":["string","object"]},"eventId":{"type":"string","maxLength":128},"kind":{"type":"string","enum":["terminal","failure","drift","missing_delivery","gate_ready","unavailable"]},"summary":{"type":"string","maxLength":4096},"evidence":{"type":"array","maxItems":16,"items":{"type":"string","maxLength":512}},"observedAtMs":{"type":"integer"},"watcherState":{"type":"string","enum":["starting","running","idle","error","stopped","unavailable"]},"lastError":{"type":"string","maxLength":1024}}},"eventId":{"type":"string","maxLength":128},"kind":{"type":"string","enum":["terminal","failure","drift","missing_delivery","gate_ready","unavailable"]},"summary":{"type":"string","maxLength":4096},"evidence":{"type":"array","maxItems":16,"items":{"type":"string","maxLength":512}},"observedAtMs":{"type":"integer"},"watcherState":{"type":"string","enum":["starting","running","idle","error","stopped","unavailable"]},"lastError":{"type":"string","maxLength":1024}},
                "required":["sessionId","watcherToken"],
                "anyOf":[{"required":["target"]},{"required":["event"]}]
            }),
        ),
        wait_tool(
            "watcher_wait",
            "Wait for bounded material Watcher reports from a durable cross-process queue for up to 60 minutes. Omitting timeoutMs selects the five-minute server-side default of 300000 ms; an explicit timeoutMs=300000 is equivalent. The separate MAX_WAIT_MS maximum remains 3600000 ms, and shorter waits require a stated reason such as a user deadline, confirmed host limit, or diagnostic purpose. The optional requestBinding is injected by the Codex PreToolUse hook and is validated against the authenticated parent capability; direct callers may omit it. A same-connection MCP notifications/cancelled request or supported host Interrupt releases only this request and preserves the durable session. watcher_cancel is separate: it durably ends the session and requires a fresh assignment for observation. If the session TTL expires, returns status=expired with empty events and the unchanged nextCursor; the wait does not consume or modify the durable event log.",
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
