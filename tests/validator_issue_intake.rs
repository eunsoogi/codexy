use std::{fs, process::Command};

use serde_json::{Value, json};
use tempfile::tempdir;

fn valid_receipt() -> Value {
    json!({
        "parent_approval": {
            "decision": "approved",
            "source_task_id": "019f49da-d44c-7e41-afde-8b1f7c58efa0"
        },
        "classification": "issue_sized_defect",
        "reproduction": {
            "decision": "supported",
            "surface_kind": "real_producer",
            "surface": "Codex app delegated child task",
            "steps": ["Open the delegated child task from its parent task"],
            "observed": "The child reaches issue creation without an approved intake receipt"
        },
        "ownership": {
            "decision": "cannot_own",
            "existing_owner": {"kind": "issue", "number": 195},
            "rationale": "Issue 195 does not own the missing intake validator"
        },
        "duplicate_search": {
            "states": ["open", "closed"],
            "search_terms": ["approved issue intake", "issue creation gate"],
            "results": [
                {"issue": 195, "state": "closed", "match_kind": "related"}
            ],
            "conclusion": {"decision": "no_duplicate"}
        },
        "necessity": {
            "decision": "thin_harness_change_required",
            "rationale": "A narrow validator closes the bypass without an issue framework"
        },
        "title": "Enforce approved issue intake",
        "body": "## Problem\nObserved bypass.\n## Scope\nOne gate.\n## Acceptance Criteria\nReject incomplete receipts.\n## Verification\nRun focused tests.",
        "labels": ["area/qa"],
        "repository_labels": ["area/qa"],
        "repository_milestones": ["1.1.2"],
        "repository_assignees": ["eunsoogi"],
        "milestone": "1.1.2",
        "assignee": "eunsoogi"
    })
}

fn run_receipt(receipt: &Value) -> std::process::Output {
    let directory = tempdir().expect("tempdir");
    let path = directory.path().join("receipt.json");
    fs::write(&path, serde_json::to_vec(receipt).expect("serialize")).expect("receipt");
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-issue-intake", "--issue-intake-file"])
        .arg(path)
        .output()
        .expect("validator")
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn cli_accepts_one_complete_candidate_with_negated_rationale() {
    assert!(run_receipt(&valid_receipt()).status.success());
}

#[test]
fn cli_rejects_missing_parent_approval() {
    let mut receipt = valid_receipt();
    receipt["parent_approval"]["decision"] = json!("rejected");
    let output = run_receipt(&receipt);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("explicit parent approval"));
}

#[test]
fn cli_rejects_non_substantive_reproduction_ownership_and_necessity() {
    let mut receipt = valid_receipt();
    receipt["reproduction"]["steps"] = json!(["x"]);
    receipt["reproduction"]["observed"] = json!("x");
    receipt["ownership"]["rationale"] = json!("x");
    receipt["necessity"]["rationale"] = json!("x");
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("substantive supported-surface reproduction"));
    assert!(text.contains("substantive ownership rationale"));
    assert!(text.contains("substantive thin-harness necessity"));
}

#[test]
fn cli_rejects_incomplete_all_state_search_and_exact_match_contradiction() {
    let mut receipt = valid_receipt();
    receipt["duplicate_search"]["states"] = json!(["open"]);
    receipt["duplicate_search"]["results"] =
        json!([{"issue": 306, "state": "open", "match_kind": "exact"}]);
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("open and closed duplicate-search states"));
    assert!(text.contains("exact duplicate result contradicts no_duplicate"));
}

#[test]
fn cli_rejects_canonical_duplicate_and_records_number() {
    let mut receipt = valid_receipt();
    receipt["duplicate_search"]["results"] =
        json!([{"issue": 195, "state": "closed", "match_kind": "exact"}]);
    receipt["duplicate_search"]["conclusion"] =
        json!({"decision": "duplicate", "canonical_issue": 195});
    let output = run_receipt(&receipt);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("canonical issue #195"));
}

#[test]
fn cli_rejects_empty_or_non_member_metadata_taxonomies() {
    let mut receipt = valid_receipt();
    receipt["repository_milestones"] = json!([]);
    receipt["repository_assignees"] = json!([]);
    receipt["labels"] = json!(["missing"]);
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("labels must be repository-valid"));
    assert!(text.contains("repository milestone taxonomy"));
    assert!(text.contains("repository assignee taxonomy"));
}

#[test]
fn cli_rejects_invalid_title_and_missing_real_heading() {
    let mut receipt = valid_receipt();
    receipt["title"] = json!("fix(agents): bypass intake");
    receipt["body"] = json!("## Problematic\n## Scope\n## Acceptance Criteria\n## Verification");
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("validated descriptive issue title"));
    assert!(text.contains("missing ## Problem"));
}

#[test]
fn cli_ignores_backtick_tilde_and_indented_code_headings() {
    for body in [
        "```markdown\n## Problem\n```\n## Scope\n## Acceptance Criteria\n## Verification",
        "~~~markdown\n## Problem\n~~~\n## Scope\n## Acceptance Criteria\n## Verification",
        "    ## Problem\n## Scope\n## Acceptance Criteria\n## Verification",
        "<!--\n## Problem\n-->\n## Scope\n## Acceptance Criteria\n## Verification",
    ] {
        let mut receipt = valid_receipt();
        receipt["body"] = json!(body);
        let output = run_receipt(&receipt);
        assert!(
            !output.status.success() && output_text(&output).contains("missing ## Problem"),
            "code heading should not satisfy required section: {body}"
        );
    }
}

#[test]
fn cli_rejects_handoff_only_classifications_and_no_change_decision() {
    for classification in ["unsupported_synthetic", "same_class_observation"] {
        let mut receipt = valid_receipt();
        receipt["classification"] = json!(classification);
        let output = run_receipt(&receipt);
        assert!(!output.status.success());
        assert!(output_text(&output).contains("handoff-only"));
    }

    let mut receipt = valid_receipt();
    receipt["necessity"]["decision"] = json!("no_change");
    let output = run_receipt(&receipt);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("no-change candidate is handoff-only"));
}

#[test]
fn cli_rejects_repeated_one_character_evidence_and_search_terms() {
    let mut receipt = valid_receipt();
    receipt["reproduction"]["surface"] = json!("x x x x x x x");
    receipt["reproduction"]["steps"] = json!(["x x x x x x x"]);
    receipt["reproduction"]["observed"] = json!("x x x x x x x");
    receipt["ownership"]["rationale"] = json!("x x x x x x x");
    receipt["necessity"]["rationale"] = json!("x x x x x x x");
    receipt["duplicate_search"]["search_terms"] = json!(["x x x x x x x"]);
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("substantive supported-surface reproduction"));
    assert!(text.contains("substantive ownership rationale"));
    assert!(text.contains("substantive thin-harness necessity"));
    assert!(text.contains("substantive duplicate-search terms"));
}

#[test]
fn cli_rejects_blank_metadata_taxonomy_entries_and_selections() {
    let mut receipt = valid_receipt();
    receipt["repository_labels"] = json!([""]);
    receipt["labels"] = json!([""]);
    receipt["repository_milestones"] = json!([""]);
    receipt["milestone"] = json!("");
    receipt["repository_assignees"] = json!([""]);
    receipt["assignee"] = json!("");
    let output = run_receipt(&receipt);
    let text = output_text(&output);
    assert!(!output.status.success());
    assert!(text.contains("labels must be repository-valid"));
    assert!(text.contains("repository milestone taxonomy"));
    assert!(text.contains("repository assignee taxonomy"));
}

#[test]
fn cli_rejects_malformed_source_task_id() {
    let mut receipt = valid_receipt();
    receipt["parent_approval"]["source_task_id"] = json!("00000000000000000000000000000000----");
    let output = run_receipt(&receipt);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("explicit parent approval"));
}

#[test]
fn cli_rejects_placeholder_source_task_id() {
    let mut receipt = valid_receipt();
    receipt["parent_approval"]["source_task_id"] = json!("00000000-0000-0000-0000-000000000000");
    let output = run_receipt(&receipt);
    assert!(!output.status.success());
    assert!(output_text(&output).contains("explicit parent approval"));
}
