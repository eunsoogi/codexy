use sha2::{Digest, Sha256};

pub(super) const AGGREGATE_SHA256: &str =
    "bfb88a44306da803fc51804541e2bde331f82716e5c39b1f9785ee8fc4e03150";

const BASELINE: [(&str, &str); 6] = [
    ("debugging", include_str!("baseline_v1/debugging.md")),
    (
        "domain-driven-development",
        include_str!("baseline_v1/domain-driven-development.md"),
    ),
    ("qa", include_str!("baseline_v1/qa.md")),
    ("refactoring", include_str!("baseline_v1/refactoring.md")),
    (
        "spec-driven-development",
        include_str!("baseline_v1/spec-driven-development.md"),
    ),
    (
        "test-driven-development",
        include_str!("baseline_v1/test-driven-development.md"),
    ),
];

pub(super) fn sources() -> Vec<(String, String)> {
    BASELINE
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect()
}

pub(super) fn diagnostics(sources: &[(String, String)]) -> Vec<String> {
    let mut errors = Vec::new();
    if sources.len() != BASELINE.len() {
        errors.push(
            "engineering baseline-v1 source inventory must contain exactly six sources".to_owned(),
        );
    }
    for ((expected_name, expected_text), (actual_name, actual_text)) in BASELINE.iter().zip(sources)
    {
        if actual_name != expected_name {
            errors.push(format!(
                "engineering baseline-v1 source name differs: expected {expected_name}"
            ));
        }
        if actual_text != expected_text {
            errors.push(format!(
                "engineering baseline-v1 bytes differ for {expected_name}"
            ));
        }
    }
    if aggregate(sources) != AGGREGATE_SHA256 {
        errors.push("engineering baseline-v1 aggregate SHA-256 differs".to_owned());
    }
    errors
}

pub(super) fn aggregate(sources: &[(String, String)]) -> String {
    let mut digest = Sha256::new();
    for (name, text) in sources {
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(text.as_bytes());
        digest.update([0]);
    }
    format!("{:x}", digest.finalize())
}
