#![allow(dead_code)]

// This registered stub currently emits no active-thread findings for any supplied evidence.
pub(super) fn check(_evidence: &str) -> Vec<String> {
    Vec::new()
}
