#![allow(dead_code)]

pub(super) const CLAUSE: &str =
    "MUST retain its active goal and plan while an implementation obligation remains";
pub(super) const SENTENCE: &str = "The owner MUST retain its active goal and plan while an implementation obligation remains, record `goal state=active` and `goal transition=none`, and return control without completing or blocking the goal.";

pub(super) fn sentence_with_clause(clause: &str) -> String {
    SENTENCE.replacen(CLAUSE, clause, 1)
}

pub(super) fn replace_clause(original: &str, replacement: &str) -> String {
    let updated = original.replacen(CLAUSE, replacement, 1);
    assert_ne!(updated, original, "heartbeat clause fixture did not mutate");
    updated
}

pub(super) fn replace_sentence(original: &str, replacement: &str) -> String {
    let updated = original.replacen(SENTENCE, replacement, 1);
    assert_ne!(updated, original, "heartbeat sentence fixture did not mutate");
    updated
}
