use super::support::{
    MATERIAL_RISK_TRIGGERS, TestResult, classification_request, expected_reviewer, resolve_profile,
};
use serde_json::json;

#[test]
fn risks_do_not_select_a_review_tier_or_force_review() -> TestResult {
    for trigger in MATERIAL_RISK_TRIGGERS
        .into_iter()
        .filter(|kind| *kind != "explicit_review_requirement")
    {
        for selected in [false, true] {
            let mut request = classification_request("middle", false, &[trigger], &[], None);
            request["review_requested"] = json!(selected);
            let route = resolve_profile(&request)?;
            assert_eq!(route["review_requested"], selected);
            assert_eq!(
                route["reviewer"],
                if selected {
                    expected_reviewer()
                } else {
                    serde_json::Value::Null
                }
            );
            assert_eq!(route["stage"], "final");
            for retired in [
                "profile",
                "full_review_limit",
                "delta_recheck_limit",
                "terminal_review_limit",
                "post_cap_re_review_limit",
            ] {
                assert!(route.get(retired).is_none());
            }
        }
    }
    Ok(())
}

#[test]
fn command_line_uses_optional_selection_without_quota() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("selection.json");
    let request = classification_request("middle", false, &["security"], &[], None);
    std::fs::write(&input, serde_json::to_vec(&request)?)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--resolve-selection", "--input"])
        .arg(&input)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout)?,
        resolve_profile(&request)?
    );
    Ok(())
}
