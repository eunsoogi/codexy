use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_recognizable_tables_fail_closed_for_every_sensitive_consumer() -> TestResult {
    for table in invalid_tables() {
        for payload in [
            "Child branch codexy/461-table was created after classification.\n",
            "Review response: parent-authored implementation commit abc123 fixed feedback\n",
            "Source thread id: parent-461\nGoal tool call: create_goal\n",
        ] {
            assert_rejected(&format!("{table}\n{payload}"))?;
        }
    }
    Ok(())
}

#[test]
fn classification_context_keeps_handoff_order_and_owner_invariants() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    let parent = canonical_table("parent-owned implementation lane");
    let external = canonical_table("external/human-owned implementation lane");
    assert_allowed(&format!(
        "{child}\nIssue: #461\nBranch: codexy/461\nWorktree path: /tmp/461\nPull request: #468\nChild branch codexy/461-table was created after classification.\n{}",
        valid_goal_receipt()
    ))?;
    for evidence in [
        format!("{parent}\nChild branch codexy/461-table was created after classification."),
        format!("{external}\nReview response: child-authored commit def456 fixed feedback"),
        format!("{child}\n{child}\nChild branch codexy/461-table was created after classification."),
        format!("{child}- Issue: #461\n- PR: #468\nChild branch codexy/461-table was created after classification."),
        format!("Child branch codexy/461-table was created.\n{child}"),
    ] {
        assert_rejected(&evidence)?;
    }
    Ok(())
}

#[test]
fn gfm_fences_and_cell_constructs_preserve_rendered_table_boundaries() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    for fenced in [
        format!("~~~markdown\n{child}~~~\n"),
        format!("````markdown\nexample:\n```\n{child}````\n"),
    ] {
        assert_allowed(&format!(
            "{fenced}Review response: parent-authored implementation commit abc123 fixed feedback\n"
        ))?;
    }
    for tools in ["cargo test \\| Sentinel", "`cargo test | Sentinel`"] {
        let table = child.replacen("goal, plan, codegraph, LSP, Sentinel", tools, 1);
        assert_allowed(&format!(
            "{table}\nChild branch codexy/461-table was created after classification.\n{}",
            valid_goal_receipt()
        ))?;
    }
    Ok(())
}

#[test]
fn indented_tables_are_code_examples_not_classification_evidence() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    for indent in ["    ", "\t"] {
        let table = child
            .lines()
            .map(|line| format!("{indent}{line}\n"))
            .collect::<String>();
        for action in [
            "Review response: parent-authored implementation commit abc123 fixed feedback\n",
            "Child branch codexy/461-table was created after classification.\n",
        ] {
            assert_allowed(&format!("{table}{action}"))?;
        }
    }
    for indent in ["", " ", "   "] {
        let table = child
            .lines()
            .map(|line| format!("{indent}{line}\n"))
            .collect::<String>();
        assert_rejected(&format!(
            "{table}Review response: parent-authored implementation commit abc123 fixed feedback\n"
        ))?;
    }
    assert_rejected(&format!(
        "{}Review response: parent-authored implementation commit abc123 fixed feedback\n",
        child.replacen("| Task classification", "- | Task classification", 1)
    ))?;
    Ok(())
}

#[test]
fn rendered_tables_exclude_html_and_retain_malformed_candidates() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    assert_allowed(&format!(
        "<!--\n{child}-->\nReview response: parent-authored implementation commit abc123 fixed feedback\n"
    ))?;
    let malformed_delimiter = child.replacen("| --- | --- |", "| --- |", 1);
    for payload in [
        "Child branch codexy/461-table was created after classification.\n",
        "Review response: parent-authored implementation commit abc123 fixed feedback\n",
        "Source thread id: parent-461\nGoal tool call: create_goal\n",
    ] {
        assert_rejected(&format!("{malformed_delimiter}\n{payload}"))?;
    }
    let inline_html = malformed_delimiter
        .replacen("Task classification", "<em>Task classification</em>", 1)
        .replacen("Owner decision", "<em>Owner decision</em>", 1);
    for payload in [
        "Child branch codexy/461-table was created after classification.\n",
        "Review response: parent-authored implementation commit abc123 fixed feedback\n",
        "Source thread id: parent-461\nGoal tool call: create_goal\n",
    ] {
        assert_rejected(&format!("{inline_html}\n{payload}"))?;
    }
    let inline_canonical = child.replacen("Task classification", "<em>Task classification</em>", 1);
    assert_allowed(&format!(
        "{inline_canonical}\nChild branch codexy/461-table was created after classification.\n{}",
        valid_goal_receipt()
    ))?;
    Ok(())
}

#[test]
fn unseparated_prose_is_part_of_the_rendered_table_and_fails_closed() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    assert_rejected(&format!(
        "{child}Child branch codexy/461-table was created after classification.\n"
    ))
}

#[test]
fn later_classification_and_line_endings_preserve_lane_invariants() -> TestResult {
    let child = canonical_table("current-thread-owned child implementation lane");
    for owner in [
        "current-thread-owned child implementation lane",
        "parent-owned implementation lane",
        "external/human-owned implementation lane",
        "child-owned or parent-owned",
    ] {
        assert_rejected(&format!(
            "Child branch codexy/461-table was created.\n{}",
            canonical_table(owner)
        ))?;
    }
    assert_rejected(&format!(
        "Child branch codexy/461-table was created.\n{}",
        child.replacen("Task classification", "Task classifications", 1)
    ))?;
    assert_allowed(&format!(
        "{child}\nChild branch codexy/461-table was created after classification.\nPull request: #999\n{}",
        canonical_table("parent-owned implementation lane")
    ))?;

    let preamble = "context\n".repeat(24);
    let cases = [
        format!("{preamble}{child}\nChild branch codexy/461-table was created after classification.\n"),
        format!("{preamble}~~~markdown\n{child}~~~\nReview response: parent-authored implementation commit abc123 fixed feedback\n"),
        format!("{preamble}<!--\n{child}-->\nReview response: parent-authored implementation commit abc123 fixed feedback\n"),
        format!("{preamble}{}Review response: parent-authored implementation commit abc123 fixed feedback\n", child.replacen("Task classification", "Task classifications", 1)),
    ];
    for lf in cases {
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(
            run_validator(&lf)?.status.success(),
            run_validator(&crlf)?.status.success(),
            "LF/CRLF changed classification evidence outcome:\n{lf}"
        );
    }
    Ok(())
}

fn invalid_tables() -> Vec<String> {
    vec![
        canonical_table("current-thread-owned child implementation lane")
            .replacen("Task classification", "Task classifications", 1),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("Task classification", "1. Task classification", 1),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("| Task classification", "+ | Task classification", 1),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("| Task classification", "- [ ] | Task classification", 1),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("| Task classification | Decision |", "| Task classification | Result |", 1),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("| --- | --- |", "| - | --- |", 1),
        canonical_table("undecided"),
        canonical_table("child-owned or parent-owned"),
        canonical_table("not child-owned implementation lane"),
        canonical_table("current-thread-owned child implementation lane")
            .replacen("| Stop/blocker | None |\n", "", 1),
    ]
}

fn canonical_table(owner: &str) -> String {
    format!(
        "| Task classification | Decision |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | workflow, validators |\n| Owner decision | {owner} |\n| Atomic scope | issue-sized |\n| Required skills | task-classification, codex-orchestration, git-workflow |\n| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |\n| First allowed action | create branch after classification |\n| Stop/blocker | None |\n"
    )
}

fn valid_goal_receipt() -> &'static str {
    "Source thread id: parent-461\nGoal control state: source_thread_id=parent-461\nGoal transition key: 461:create_goal:proof\nParent goal pre-delivery: operation=create_goal; parent task=parent-461; delivery=confirmed; task surface=codex task/thread; issue=#461; plan step=implement; branch=codexy/461; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=461:create_goal:proof\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-461; delivery=confirmed; task surface=codex task/thread; transition key=461:create_goal:proof\n"
}

fn assert_rejected(evidence: &str) -> TestResult {
    assert!(
        !run_validator(evidence)?.status.success(),
        "validator accepted invalid classification evidence:\n{evidence}"
    );
    Ok(())
}

fn assert_allowed(evidence: &str) -> TestResult {
    assert!(
        run_validator(evidence)?.status.success(),
        "validator rejected canonical classification evidence:\n{evidence}"
    );
    Ok(())
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}
