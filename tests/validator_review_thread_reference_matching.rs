use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_comment_url_prefix_collision_for_thread_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for https://github.com/eunsoogi/codexy/pull/130#discussion_r10.\n",
        two_prefix_collision_review_threads_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should not let discussion_r10 cover discussion_r1 by substring\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOPrefixOne"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_later_comment_url_for_thread_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for https://github.com/eunsoogi/codexy/pull/134#discussion_r3435371456. Preventive adjacent review no-change rationale: inspected functions review_thread_resolution::thread_referenced and tests validator_review_thread_reference_matching; invariants hold because sibling URL variants still require exact reference boundaries.\n",
        multi_comment_review_thread_pr_state(),
    )?;

    assert!(
        output.status.success(),
        "validator should match later comment URLs in a review thread\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            pr_state_path.to_str().ok_or("pr state path")?,
        ])
        .output()?)
}

fn two_prefix_collision_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOPrefixOne",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r1"}]}
                },
                {
                    "id": "PRRT_kwDOPrefixTen",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r10"}]}
                }
            ]
        }
    }"#
}

fn multi_comment_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOS6i-_86KhdRE",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {
                        "nodes": [
                            {"url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435371000"},
                            {"url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435371456"}
                        ]
                    }
                }
            ]
        }
    }"#
}
