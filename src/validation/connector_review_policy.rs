use std::path::Path;

use crate::paths::display_relative;

const FORBIDDEN_MARKERS: [&str; 6] = [
    "@codex review",
    "codex review",
    "codex-review",
    "codex connector review",
    "codex connector output",
    "chatgpt-codex-connector",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if FORBIDDEN_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        errors.push(format!(
            "{} Codex connector review policy is not allowed",
            display_relative(path)
        ));
    }
}
