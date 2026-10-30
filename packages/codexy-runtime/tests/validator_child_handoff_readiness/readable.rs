use super::*;

#[test]
fn readable_readiness_claim_must_match_dirty_status_and_head() -> TestResult {
    let table = "Child handoff:\n| Field | Value |\n| --- | --- |\n| Status | ready for parent handoff |\n| Branch | clean and pushed |\n| Head | 068dbb247b7755035223c91ee39f26830f3c1609 |\n";
    let state = pr_state_with(
        r#""mergeStateStatus":"CLEAN","headRefOid":"1111111111111111111111111111111111111111","worktreeStatus":"M changed","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
    );
    let output = validate_handoff_with_pr_state(table, &state)?;
    assert!(
        !output.status.success(),
        "table readiness must not bypass current Git proof"
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("dirty") || error.contains("headRefOid"),
        "{error}"
    );
    let stale = pr_state_with(
        r###""mergeStateStatus":"CLEAN","headRefName":"topic","headRefOid":"1111111111111111111111111111111111111111","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## topic...origin/topic","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
    );
    let output = validate_handoff_with_pr_state(table, &stale)?;
    assert!(!output.status.success(), "table must reject stale head");
    let matching = stale.replace(
        "1111111111111111111111111111111111111111",
        "068dbb247b7755035223c91ee39f26830f3c1609",
    );
    let valid = validate_handoff_with_pr_state(table, &matching)?;
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let contradictory = table.replace(
        "| Branch | clean and pushed |",
        "| Branch | clean and pushed |\n| Pushed | no |",
    );
    assert!(
        !validate_handoff_with_pr_state(&contradictory, &matching)?
            .status
            .success()
    );
    Ok(())
}

#[test]
fn readiness_tables_preserve_negative_and_wrapped_status() -> TestResult {
    let negative = "Child handoff:\n| Field | Value |\n| --- | --- |\n| PR ready | no |\n| Parent can merge | no |\n| Pushed | no |\n";
    let dirty = pr_state_with(
        r#""mergeStateStatus":"DIRTY","headRefOid":"1111111111111111111111111111111111111111","worktreeStatus":"M changed","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
    );
    let output = validate_handoff_with_pr_state(negative, &dirty)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wrapped = "Child handoff:\n| Field | Value |\n| --- | --- |\n| Status | ready for parent\nhandoff |\n| Branch | clean and pushed |\n| Head | 068dbb247b7755035223c91ee39f26830f3c1609 |\n";
    assert!(
        !validate_handoff_with_pr_state(wrapped, &dirty)?
            .status
            .success()
    );
    Ok(())
}
