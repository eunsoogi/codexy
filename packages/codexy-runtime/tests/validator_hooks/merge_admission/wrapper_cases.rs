use super::*;

#[cfg(unix)]
#[test]
fn canonical_wrapper_binds_validated_message_to_merge_payload() -> TestResult {
    let message = "fix(workflow): require intent (#128)\n\nFixes #503\n";
    let subject = "fix(workflow): require intent (#128)";
    let (output, merged, _) = wrapper_with_payload(
        &github_plugin_root(),
        state(),
        false,
        message,
        subject,
        "Fixes #503\n",
        false,
    )?;
    assert!(
        output.status.success(),
        "exact payload rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(merged, "exact payload did not reach merge");
    for (actual_subject, actual_body) in [
        ("fix: malformed subject", "Fixes #503\n"),
        (subject, "This body does not close #503\n"),
    ] {
        let (output, merged, _) = wrapper_with_payload(
            &github_plugin_root(),
            state(),
            false,
            message,
            actual_subject,
            actual_body,
            false,
        )?;
        assert!(
            !output.status.success(),
            "decoy message admitted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!merged, "decoy message reached merge");
    }
    let invalid = "fix: malformed subject\n\nFixes #503\n";
    let (output, merged, _) = wrapper_with_payload(
        &github_plugin_root(),
        state(),
        false,
        invalid,
        "fix: malformed subject",
        "Fixes #503\n",
        false,
    )?;
    assert!(!output.status.success(), "malformed exact payload admitted");
    assert!(!merged, "malformed exact payload reached merge");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_gh_uses_immutable_body_snapshot() -> TestResult {
    let (output, merged, body) = wrapper_with_payload(
        &github_plugin_root(),
        state(),
        false,
        "fix(workflow): require intent (#128)\n\nFixes #503\n",
        "fix(workflow): require intent (#128)",
        "Fixes #503\n",
        true,
    )?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(merged, "immutable body did not reach merge");
    assert_eq!(
        body, "Fixes #503\n",
        "post-admission mutation changed gh body"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_rejects_bad_github_authorization_captures() -> TestResult {
    let duplicate = state().replacen(
        "]}",
        r#",{"id":"IC_replay","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-130","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"MEMBER"}]}"#,
        1,
    );
    for capture in [
        state().replacen("eunsoogi/codexy", "openai/codex", 1),
        state().replacen("\"number\":128", "\"number\":127", 1),
        state().replacen("32b03a210b3defb2d29dd352283ea2488e60d893", "stale-head", 1),
        state().replacen("AUTHORIZE REPOSITORY", "DO NOT AUTHORIZE REPOSITORY", 1),
        duplicate,
    ] {
        let (output, merged) = wrapper_output(&github_plugin_root(), &capture, false)?;
        assert!(
            !output.status.success(),
            "bad capture admitted: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!merged, "bad capture reached merge");
    }
    let (output, merged) = wrapper_output(&github_plugin_root(), state(), true)?;
    assert!(!output.status.success(), "GitHub API failure admitted");
    assert!(!merged, "GitHub API failure reached merge");
    Ok(())
}
