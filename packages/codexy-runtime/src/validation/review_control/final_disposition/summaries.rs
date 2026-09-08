use std::collections::HashSet;

use serde_json::{Map, Value};

use super::fields::{reject_unknown, text};

pub(super) fn check(object: &Map<String, Value>, expected: &[String]) -> Result<(), String> {
    let summaries = object
        .get("public_summaries")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "final disposition evidence refresh must list public_summaries".to_owned()
        })?;
    if summaries.len() != expected.len() {
        return Err(
            "final disposition public summaries must cover evidence findings exactly once".into(),
        );
    }
    let mut seen = HashSet::new();
    for summary in summaries {
        let summary = summary
            .as_object()
            .ok_or_else(|| "final disposition public summary must be an object".to_owned())?;
        reject_unknown(
            summary,
            &["finding_id", "status", "case_hash"],
            "public summary",
        )?;
        let id = text(summary, "finding_id", "public summary")?;
        if !expected.iter().any(|expected| expected == id) || !seen.insert(id) {
            return Err("final disposition public summary finding coverage is invalid".into());
        }
        if text(summary, "status", "public summary")? != "PASS" {
            return Err("final disposition public summary status must be PASS".into());
        }
        let hash = text(summary, "case_hash", "public summary")?;
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("final disposition public summary case_hash must be SHA-256".into());
        }
    }
    Ok(())
}
