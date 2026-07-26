use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use super::audit_math::checked_add;

#[derive(Debug, Serialize)]
pub(super) struct Report {
    pub(super) session_count: usize,
    pub(super) duplicate_events_skipped: u64,
    pub(super) sessions: Vec<SessionReport>,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct AuditWindow {
    policy: &'static str,
    pub(super) records_observed: u64,
    pub(super) turn_events: u64,
}

#[derive(Debug, Default, Serialize)]
struct ToolFamilyReport {
    calls: u64,
    input_bytes: u64,
    output_bytes: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct SessionReport {
    pub(super) session_id: String,
    byte_provenance: &'static str,
    pub(super) size_bytes: u64,
    pub(super) latest_cumulative_tokens: u64,
    pub(super) recent_turn_average_tokens: u64,
    pub(super) window: AuditWindow,
    pub(super) tool_calls: BTreeMap<String, u64>,
    pub(super) tool_input_bytes: BTreeMap<String, u64>,
    pub(super) tool_output_bytes: BTreeMap<String, u64>,
    tool_families: BTreeMap<String, ToolFamilyReport>,
    pub(super) event_ids: Vec<String>,
    pub(super) event_ids_truncated: bool,
    #[serde(skip)]
    pub(super) cumulative_tokens: Vec<u64>,
}

impl SessionReport {
    pub(super) fn new(session_id: String, byte_provenance: &'static str) -> Self {
        Self {
            session_id,
            byte_provenance,
            size_bytes: 0,
            latest_cumulative_tokens: 0,
            recent_turn_average_tokens: 0,
            window: AuditWindow {
                policy: "records",
                ..AuditWindow::default()
            },
            tool_calls: BTreeMap::new(),
            tool_input_bytes: BTreeMap::new(),
            tool_output_bytes: BTreeMap::new(),
            tool_families: BTreeMap::new(),
            event_ids: Vec::new(),
            event_ids_truncated: false,
            cumulative_tokens: Vec::new(),
        }
    }

    pub(super) fn record_event_id(&mut self, event_id: String) {
        if self.event_ids.len() < 64 {
            self.event_ids.push(event_id);
        } else {
            self.event_ids_truncated = true;
        }
    }

    pub(super) fn finalize_tool_families(&mut self) -> Result<()> {
        for (tool, calls) in &self.tool_calls {
            let Some(family) = tool_family(tool) else {
                continue;
            };
            let report = self.tool_families.entry(family.to_owned()).or_default();
            report.calls = checked_add(report.calls, *calls, "tool family calls")?;
            report.input_bytes = checked_add(
                report.input_bytes,
                self.tool_input_bytes.get(tool).copied().unwrap_or_default(),
                "tool family input bytes",
            )?;
            report.output_bytes = checked_add(
                report.output_bytes,
                self.tool_output_bytes
                    .get(tool)
                    .copied()
                    .unwrap_or_default(),
                "tool family output bytes",
            )?;
        }
        Ok(())
    }
}

fn tool_family(tool: &str) -> Option<&'static str> {
    match tool {
        "exec" | "exec_command" | "functions.exec" | "functions.exec_command" => Some("exec"),
        "wait" | "wait_agent" | "functions.wait" | "functions.wait_agent" => Some("wait"),
        _ => None,
    }
}
