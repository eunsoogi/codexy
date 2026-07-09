use super::completion_handoff_pending_worktree_text::has_any;

pub(super) fn colon_starts_lifecycle_entry(text: &str, colon: usize) -> bool {
    let suffix = &text[colon + 1..];
    let next_local = suffix.find("local:").unwrap_or(suffix.len());
    let entry = &suffix[..next_local];
    has_any(
        entry,
        "failed setup|setup failed|surfaced thread id|thread id|bounded timeout|bounded wait|remains unresolved|still unresolved|not visible|not surfaced",
    )
}

pub(super) fn bounded_search_evidence_text(text: &str) -> &str {
    text.split("metadata:").next().unwrap_or(text)
}
