use crate::support::wiki_minimal_contract_markdown::Document;

pub(crate) const WORKFLOWS: &[(&str, &str)] = &[
    ("init", "Keep"),
    ("ingest", "Keep"),
    ("ingest-collection", "Merge"),
    ("collect", "Remove"),
    ("compile", "Keep"),
    ("query", "Keep"),
    ("refresh", "Keep"),
    ("lint", "Keep"),
    ("librarian", "Merge"),
    ("audit", "Merge"),
    ("research", "Merge"),
    ("output", "Merge"),
    ("plan", "Remove"),
    ("project", "Remove"),
    ("inventory", "Remove"),
    ("dataset", "Remove"),
    ("archive", "Remove"),
    ("ll", "Remove"),
    ("assess", "Merge"),
    ("retract", "Merge"),
    ("thesis", "Merge"),
    ("status", "Remove"),
    ("session", "Remove"),
    ("session-capture", "Remove"),
    ("rehydrate", "Remove"),
    ("session-promote", "Merge"),
    ("feedback", "Remove"),
    ("feedback-capture", "Remove"),
    ("feedback-promote", "Merge"),
];

pub(crate) const ASSIGNMENTS: &[(&str, &str)] = &[
    ("query.max_index_files", "3"),
    ("query.max_article_files", "8"),
    ("query.max_index_file_bytes", "4000"),
    ("query.max_article_file_bytes", "4000"),
    ("query.max_total_bytes", "48000"),
    (
        "freshness.threshold",
        "config.md freshness_threshold (default 70)",
    ),
    ("freshness.hot_half_life_days", "30"),
    ("freshness.warm_half_life_days", "90"),
    ("freshness.cold_half_life_days", "365"),
    ("freshness.decay", "25 * 0.5^(age_days / half_life_days)"),
    (
        "freshness.compilation_date",
        "updated when valid; otherwise created when valid; otherwise 0",
    ),
    (
        "freshness.source_age",
        "average(age_days across resolvable sources)",
    ),
    (
        "freshness.source_chain",
        "25 * resolvable_sources / total_sources",
    ),
    (
        "freshness.score",
        "round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)",
    ),
    ("freshness.future_date", "0"),
    (
        "freshness.conversation",
        "min(100, 2 * (verification + compilation))",
    ),
];

pub(crate) fn validate_contract(text: &str) -> Result<(), String> {
    let document = Document::parse(text)?;
    let essential = document.section("## Essential contract")?;
    let measurable = document.section("## Measurable criteria")?;
    let workflows =
        document.workflow_rows(&document.section("## Current workflow disposition")?)?;
    let measures =
        document.assignments(&document.child(&measurable, "### Machine-checkable limits")?)?;
    for title in [
        "### Ingest",
        "### Compile",
        "### Query",
        "### Refresh and provenance",
        "### Bounded context",
    ] {
        document.child(&essential, title)?;
    }
    for title in [
        "### Context efficiency",
        "### Traceability",
        "### Freshness",
    ] {
        document.child(&measurable, title)?;
    }
    exact_workflows(&workflows)?;
    exact_assignments(&measures)
}

fn exact_workflows(workflows: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    if workflows.len() != WORKFLOWS.len() {
        return Err("incomplete workflow inventory".into());
    }
    for (workflow, disposition) in WORKFLOWS {
        if workflows.get(*workflow).map(String::as_str) != Some(*disposition) {
            return Err(format!("invalid disposition for {workflow}"));
        }
    }
    Ok(())
}

fn exact_assignments(measures: &std::collections::BTreeMap<String, String>) -> Result<(), String> {
    if measures.len() != ASSIGNMENTS.len() {
        return Err("unexpected or incomplete assignment inventory".into());
    }
    for (key, value) in ASSIGNMENTS {
        if measures.get(*key).map(String::as_str) != Some(*value) {
            return Err(format!("invalid assignment {key}"));
        }
    }
    Ok(())
}
