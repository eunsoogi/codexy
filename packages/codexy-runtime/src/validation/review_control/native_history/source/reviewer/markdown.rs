use serde_json::{Map, Value, json};

#[path = "markdown/context.rs"]
mod context;
#[path = "markdown/control.rs"]
mod control;
#[path = "markdown/findings.rs"]
mod findings;
#[path = "markdown/head.rs"]
mod head;
#[path = "markdown/result.rs"]
mod result;
#[path = "markdown/settings.rs"]
mod settings;

pub(super) fn line_ranges(raw: &str) -> Vec<(usize, usize, &str)> {
    context::line_ranges(raw)
}

pub(super) fn operative_lines(raw: &str) -> Vec<(usize, usize, &str)> {
    context::operative_lines(raw)
}

pub(super) fn operative_line_indices(lines: &[(usize, usize, &str)]) -> Vec<usize> {
    context::operative_line_indices(lines)
}

pub(super) fn heading(line: &str) -> Option<(usize, &str)> {
    context::heading(line)
}

pub(super) fn candidate(raw: &str) -> Result<Option<Map<String, Value>>, String> {
    let mut result = Map::new();
    let mut spans = Map::new();
    if let Some(kind) = head::terminal_history_kind(raw)? {
        spans.insert("kind".into(), kind.span());
        result.insert("kind".into(), Value::String(kind.value));
    }
    if let Some(head) = head::reviewed_head(raw)? {
        let span = head.span();
        result.insert("reviewed_head".into(), Value::String(head.value));
        spans.insert("reviewed_head".into(), span);
    }
    if let Some(terminal) = control::terminal_result(raw)? {
        let span = terminal.span();
        result.insert("terminal_result".into(), Value::String(terminal.value));
        spans.insert("terminal_result".into(), span);
    }
    if let Some(setting) = settings::reviewer_setting(raw)? {
        let span = setting.span();
        result.insert("model".into(), Value::String(setting.model));
        result.insert("reasoningEffort".into(), Value::String(setting.effort));
        spans.insert("reviewer".into(), span);
    }
    let findings = findings::values(raw)?;
    if !findings.is_empty() {
        result.insert("findings".into(), Value::Array(findings));
        spans.insert(
            "findings".into(),
            json!({"source": "final_message_markdown"}),
        );
    }
    if result.is_empty() {
        return Ok(None);
    }
    result.insert("sourceSpans".into(), Value::Object(spans));
    Ok(Some(result))
}

struct Located {
    value: String,
    start: usize,
    end: usize,
}

impl Located {
    fn span(&self) -> Value {
        json!({
            "source": "final_message_markdown",
            "start": self.start,
            "end": self.end,
            "coordinate": "utf8_bytes"
        })
    }
}

struct Setting {
    model: String,
    effort: String,
    start: usize,
    end: usize,
}

impl Setting {
    fn span(&self) -> Value {
        json!({
            "source": "final_message_markdown",
            "start": self.start,
            "end": self.end,
            "coordinate": "utf8_bytes"
        })
    }
}

pub(super) fn labelled_head(line: &str) -> Result<Option<(String, usize)>, String> {
    head::labelled_head(line)
}

pub(super) fn result_value(value: &str) -> Option<(String, usize)> {
    control::result_value(value)
}
