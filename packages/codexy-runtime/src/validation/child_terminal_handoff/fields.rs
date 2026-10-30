use super::*;

pub(super) fn confirmed_handoff(line: &str, source: Option<&str>) -> bool {
    // A receipt must identify both actual tasks; placeholders are not transport evidence.
    ["parent task", "child task"].iter().all(|name| {
        field(line, name)
            .is_some_and(|task| !is_local_task_target(task) && !is_placeholder_task(task))
    }) && source.is_none_or(|expected| field(line, "parent task") == Some(expected))
        && delivery_supported(line)
        && REQUIRED_FIELDS
            .iter()
            .all(|name| field(line, name).is_some_and(valid_value))
}

// Omitted permission remains legacy-compatible; duplicate fields are not omission.
// Host final-result submission is an honest fallback, never a confirmed message receipt.
fn delivery_supported(line: &str) -> bool {
    (field(line, "delivery") == Some("confirmed")
        && field(line, "task surface") == Some("codex task/thread")
        && field(line, "message permission").map_or_else(
            || {
                !line
                    .split(if line.contains('\u{1f}') {
                        '\u{1f}'
                    } else {
                        ';'
                    })
                    .any(|part| part.trim().starts_with("message permission="))
            },
            |permission| permission == "granted",
        ))
        || (line.starts_with("idle wait handoff:")
            && field(line, "delivery") == Some("unconfirmed")
            && field(line, "task surface") == Some("host terminal result")
            && field(line, "message permission") == Some("absent"))
}

fn is_placeholder_task(value: &str) -> bool {
    matches!(
        value,
        "codex task/thread" | "parent task" | "child task" | "task/thread"
    )
}

pub(in crate::validation) fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    let mut values = line
        .split_once(": ")
        .map_or(line, |(_, value)| value)
        .split(if line.contains('\u{1f}') {
            '\u{1f}'
        } else {
            ';'
        })
        .map(str::trim)
        .filter_map(|part| part.strip_prefix(&prefix));
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

pub(super) fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "false" | "unavailable" | "none")
        && !value.contains(" unavailable")
}
