use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;

use super::{economics_package, repository};

pub(super) const UNAVAILABLE_REASON: &str = "review economics observation unavailable: no callable verifier can independently read the original Codex task/tool record and no platform-issued signature validator with a non-caller-controlled trust root is exposed";

pub(super) fn check(_plugin_root: &Path, _repository_root: &Path, input: &str) -> Result<()> {
    let report: Value = serde_json::from_str(input)?;
    economics_package::load(_plugin_root)?;
    repository::current_head(_repository_root)?;
    if report["status"] == "unavailable" {
        validate_unavailable(&report)?;
    }
    bail!(UNAVAILABLE_REASON)
}

fn validate_unavailable(report: &Value) -> Result<()> {
    let fields = [
        "head_oid",
        "tree_oid",
        "policy_sha256",
        "corpus_sha256",
        "package_sha256",
        "baseline_sha256",
        "provenance",
    ];
    if report["schema"] != "codexy.review-economics.v2"
        || fields.iter().any(|field| !report[*field].is_null())
        || !report["lanes"].as_array().is_some_and(Vec::is_empty)
        || text(report, "reason").is_err()
    {
        bail!(
            "unavailable review economics must record null measurements without an acceptance claim"
        );
    }
    Ok(())
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing string field: {key}"))
}
