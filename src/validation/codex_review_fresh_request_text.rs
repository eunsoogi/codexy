pub(super) fn is_connector_footer(clause: &str) -> bool {
    if is_listed_trigger_footer(clause) {
        return true;
    }
    matches!(
        strip_markdown_quote_markers(clause),
        "comment \"@codex review\" to request another review"
            | "comment '@codex review' to request another review"
            | "comment `@codex review` to request another review"
    )
}

fn is_listed_trigger_footer(clause: &str) -> bool {
    let line = strip_markdown_quote_prefixes(clause);
    ["- ", "* ", "+ "]
        .iter()
        .filter_map(|marker| line.strip_prefix(marker))
        .map(str::trim)
        .map(trim_task_marker)
        .any(|line| line == "comment \"@codex review\"")
}

pub(super) fn has_negative_request_status(clause: &str) -> bool {
    let clause = strip_markdown_quote_markers(clause);
    let Some(value) = request_status_value_for_clause(clause) else {
        return false;
    };
    [
        "none",
        "none yet",
        "not requested",
        "not yet requested",
        "no",
        "false",
        "not applicable",
        "n/a",
    ]
    .iter()
    .any(|negative| {
        value.strip_prefix(negative).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|character| !character.is_ascii_alphanumeric())
        })
    })
}

fn request_status_value_for_clause(clause: &str) -> Option<&str> {
    std::iter::successors(Some(clause), |clause| {
        clause
            .split_once(':')
            .map(|(_, suffix)| suffix.trim_start())
    })
    .find_map(|clause| {
        [
            "current-head codex review request",
            "current-head @codex review request",
            "current head codex review request",
            "current head @codex review request",
            "codex review request",
            "@codex review request",
        ]
        .iter()
        .filter_map(|label| clause.strip_prefix(label))
        .find_map(request_status_value)
    })
}

fn request_status_value(suffix: &str) -> Option<&str> {
    suffix
        .trim_start()
        .strip_prefix([':', '?', '='])
        .map(str::trim_start)
}

fn strip_markdown_quote_markers(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        if let Some(rest) = text
            .strip_prefix('>')
            .or_else(|| text.strip_prefix("&gt;"))
            .or_else(|| text.strip_prefix("&gt"))
            .or_else(|| text.strip_prefix("- "))
            .or_else(|| text.strip_prefix("* "))
            .or_else(|| text.strip_prefix("+ "))
            .or_else(|| strip_task_marker(text))
        {
            text = rest;
        } else {
            return text;
        }
    }
}

fn strip_markdown_quote_prefixes(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        if let Some(rest) = text
            .strip_prefix('>')
            .or_else(|| text.strip_prefix("&gt;"))
            .or_else(|| text.strip_prefix("&gt"))
        {
            text = rest;
        } else {
            return text;
        }
    }
}

fn trim_task_marker(text: &str) -> &str {
    ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|marker| text.strip_prefix(marker))
        .unwrap_or(text)
}

fn strip_task_marker(text: &str) -> Option<&str> {
    ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|marker| text.strip_prefix(marker))
}
