use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[path = "native_919_pages.rs"]
mod pages;

pub(crate) const ISSUE: u64 = 919;
pub(crate) const PULL_REQUEST: u64 = 989;
pub(crate) const BASE: &str = "fa12bcacf84e00ec4a66f4d8cd9d9f1693f83d9c";
pub(crate) const FULL_HEAD: &str = "e8df6c7a1e2a43c544f106c08e9ce140f0e9f6ca";
pub(crate) const DELTA_HEAD: &str = "6419fdeba270c2b889ee1986bdcda8642d084458";
pub(crate) const CURRENT_HEAD: &str = "9e431bcfcae06227ab0c9ce0986f7edea709dcc6";
pub(crate) const FULL_EVENT: &str = "strict-full-1";
pub(crate) const DELTA_EVENT: &str = "strict-delta-1";
pub(crate) const REQUIRED_EVENT: &str = "strict-required-head-1";
pub(crate) const REMAINING_FINDING: &str = "919-canonical-execution-provenance";

pub(crate) fn request_919() -> Value {
    json!({
        "schema": "codexy.review-control-native-history-request.v1",
        "target": {
            "repository": "eunsoogi/codexy",
            "owningIssue": ISSUE,
            "pullRequest": PULL_REQUEST,
            "phase": "post_pr"
        },
        "owner": {"pages": pages::owner_pages()},
        "reviewer": {"pages": pages::reviewer_pages()}
    })
}

pub(crate) fn current_snapshot_919() -> Value {
    json!({
        "repository": "eunsoogi/codexy",
        "number": PULL_REQUEST,
        "baseRefName": "main",
        "baseRefOid": BASE,
        "headRefOid": CURRENT_HEAD,
        "url": "https://github.com/eunsoogi/codexy/pull/989",
        "createdAtEpoch": 50,
        "reviewProfile": "strict",
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []},
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "owningIssue": {
                "repository": "eunsoogi/codexy",
                "number": ISSUE,
                "url": "https://github.com/eunsoogi/codexy/issues/919",
                "association": "owner-assignment"
            }
        }
    })
}

pub(crate) fn recover_919() -> TestResult<(Value, Value)> {
    let temporary = tempfile::tempdir()?;
    let current_path = temporary.path().join("current.json");
    let input_path = temporary.path().join("native-history.json");
    let output_path = temporary.path().join("recovered.json");
    let current = current_snapshot_919();
    fs::write(&current_path, serde_json::to_vec(&current)?)?;
    fs::write(&input_path, serde_json::to_vec(&request_919())?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--recover-native-review-history", "--current-pr-state-file"])
        .arg(&current_path)
        .args(["--input"])
        .arg(&input_path)
        .args(["--output"])
        .arg(&output_path)
        .output()?;
    if !result.status.success() {
        return Err(format!(
            "#919 native history recovery failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    let recovered: Value = serde_json::from_slice(&fs::read(output_path)?)?;
    let mut current_with_receipt = current;
    current_with_receipt["nativeHistoryRecovery"] = recovered["nativeHistoryRecovery"].clone();
    Ok((recovered, current_with_receipt))
}

pub(crate) fn final_control_919(recovered: &Value) -> TestResult<Value> {
    let mut control = recovered
        .get("reviewControl")
        .cloned()
        .ok_or("#919 recovered review control")?;
    let policy_reviewer = control["reviewer"].clone();
    let source_reviewer = control["terminal_review_history"][1]["reviewer"].clone();
    let finding = control["terminal_review_history"][1]["unresolved_findings"][1].clone();
    let qualifying_finding_ids = control["terminal_review_history"][1]["unresolved_findings"]
        .as_array()
        .ok_or("#919 recovered delta findings")?
        .iter()
        .map(|finding| {
            finding
                .get("id")
                .cloned()
                .ok_or("#919 recovered delta finding id")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let required_findings = json!([finding]);
    let history = control["terminal_review_history"]
        .as_array_mut()
        .ok_or("#919 recovered terminal history")?;
    history.push(json!({
        "id": REQUIRED_EVENT,
        "kind": "required_current_head",
        "reviewer": source_reviewer,
        "policy_reviewer": policy_reviewer,
        "source_reviewer": history[1]["reviewer"],
        "reviewed_head": CURRENT_HEAD,
        "terminal_result": "BLOCK",
        "unresolved_findings": required_findings
    }));
    let object = control.as_object_mut().ok_or("#919 control object")?;
    object.remove("native_history_recovery");
    object.insert("reviewed_head".into(), json!(CURRENT_HEAD));
    object.insert("terminal_result".into(), json!("BLOCK"));
    object.insert("unresolved_findings".into(), required_findings);
    object.insert("terminal_review_count".into(), json!(3));
    object.insert("post_cap_re_review".into(), json!({
        "reason": "in_scope_contract_root_repair",
        "prior_reviewed_head": DELTA_HEAD,
        "qualifying_change": {
            "from_head": DELTA_HEAD,
            "to_head": CURRENT_HEAD,
            "evidence_commit": CURRENT_HEAD,
            "finding_ids": qualifying_finding_ids
        }
    }));
    object.insert("final_disposition".into(), json!({
        "schema": "codexy.review-control-final-disposition.v1",
        "kind": "third_block_repair",
        "review_event_id": REQUIRED_EVENT,
        "source_head": CURRENT_HEAD,
        "head_oid": CURRENT_HEAD,
        "base_oid": BASE,
        "addressed_finding_ids": [REMAINING_FINDING],
        "remaining_finding_ids": [],
        "evidence_refresh": {
            "proof_head": CURRENT_HEAD,
            "finding_ids": [REMAINING_FINDING],
            "public_summaries": [{
                "finding_id": REMAINING_FINDING,
                "status": "PASS",
                "case_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }]
        }
    }));
    Ok(control)
}
