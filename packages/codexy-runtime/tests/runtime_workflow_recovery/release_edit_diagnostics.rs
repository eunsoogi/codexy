//! Safe event facts must diagnose failures without relaxing the release integrity gate.
use std::{fs, os::unix::fs::PermissionsExt, process::Command};
use serde_json::json;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn release_edit_diagnostics_bound_and_redact_untrusted_event_facts() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    // No network: a body-only event reaches this fixture, then fails baseline identity admission.
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
printf '%s\n' '{"assets":[]}'
"#)?;
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755))?;
    let event = temp.path().join("event.json");
    let cases = [
        (json!({"action":"edited","changes":{"body":{"from":"PRIVATE_BODY"}},"release":{"id":42}}), "baseline_asset_id", "body"),
        (json!({"action":"edited","changes":{"body":{"from":"PRIVATE_BODY"},"name":{"from":"PRIVATE_NAME"}},"release":{"id":42}}), "event_changes", "body,name"),
        (json!({"action":"edited","changes":{},"release":{"id":42}}), "event_changes", "none"),
        (json!({"action":"edited","changes":[],"release":{"id":42}}), "event_changes", "invalid"),
        (json!({"action":"edited","release":{"id":42}}), "event_changes", "invalid"),
        (json!({"action":"edited","changes":null,"release":{"id":42}}), "event_changes", "invalid"),
        (json!({"action":"edited","changes":"PRIVATE_VALUE","release":{"id":42}}), "event_changes", "invalid"),
        (json!({"action":"PRIVATE_ACTION\n::error::injected","changes":{"PRIVATE_KEY\n::error::injected":{}},"release":{"id":42}}), "event_action", "other"),
        (json!({"action":"edited","changes":{"body":{}},"release":"PRIVATE_RELEASE"}), "release_id", "body"),
        (json!({"action":"edited","changes":{"body":{}},"release":{"id":"PRIVATE_ID"}}), "release_id", "body"),
    ];
    for (payload, stage, keys) in cases {
        fs::write(&event, serde_json::to_vec(&payload)?)?;
        let output = Command::new(root.join("scripts/verify-release-edit-baseline"))
            .current_dir(temp.path()).env("GITHUB_EVENT_PATH", &event)
            .env("GITHUB_REPOSITORY", "example/fixture")
            .env("GH_TOKEN", "PRIVATE_TOKEN")
            .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH")?))
            .output()?;
        assert!(!output.status.success());
        let diagnostic = String::from_utf8(output.stderr)?;
        assert!(diagnostic.contains(&format!("stage={stage}")), "{diagnostic}");
        assert!(diagnostic.contains(&format!("changes_keys={keys}")), "{diagnostic}");
        assert!(!diagnostic.contains("PRIVATE"), "{diagnostic}");
        assert!(!diagnostic.contains("::error::"), "{diagnostic}");
        assert_eq!(diagnostic.lines().count(), 1, "{diagnostic}");
        assert!(diagnostic.len() < 400);
        assert!(output.stdout.is_empty());
    }
    // Oversized/unknown keys never become raw log output or workflow commands.
    let changes: serde_json::Map<String, serde_json::Value> = (0..100)
        .map(|i| (format!("PRIVATE_{}_{i}", "x".repeat(100)), json!({"from":"PRIVATE_BODY"})))
        .collect();
    fs::write(&event, serde_json::to_vec(&json!({"action":"edited","changes":changes,"release":{"id":42}}))?)?;
    let output = Command::new(root.join("scripts/verify-release-edit-baseline"))
        .current_dir(temp.path()).env("GITHUB_EVENT_PATH", &event)
        .env("GITHUB_REPOSITORY", "example/fixture").output()?;
    let diagnostic = String::from_utf8(output.stderr)?;
    assert!(!output.status.success());
    assert!(diagnostic.contains("truncated"), "{diagnostic}");
    assert!(!diagnostic.contains("PRIVATE"));
    assert!(diagnostic.len() < 400);
    // Parser diagnostics must not echo malformed JSON values or attacker log commands.
    fs::write(&event, "{PRIVATE_BODY ::error:: PRIVATE_TOKEN")?;
    let output = Command::new(root.join("scripts/verify-release-edit-baseline"))
        .current_dir(temp.path()).env("GITHUB_EVENT_PATH", &event)
        .env("GITHUB_REPOSITORY", "example/fixture").output()?;
    let diagnostic = String::from_utf8(output.stderr)?;
    assert!(!output.status.success());
    assert!(diagnostic.contains("stage=event_action action=invalid changes_keys=invalid"), "{diagnostic}");
    assert!(!diagnostic.contains("PRIVATE"));
    assert_eq!(diagnostic.lines().count(), 1);
    fs::write(&event, "{\"action\":\"edited\",\"changes\":{}}\n".repeat(100))?;
    let output = Command::new(root.join("scripts/verify-release-edit-baseline"))
        .current_dir(temp.path()).env("GITHUB_EVENT_PATH", &event)
        .env("GITHUB_REPOSITORY", "example/fixture").output()?;
    let diagnostic = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(diagnostic.contains("action=invalid changes_keys=invalid"), "{diagnostic}");
    assert_eq!(diagnostic.lines().count(), 1);
    assert!(diagnostic.len() < 400);
    Ok(())
}

// Shared with the authenticated-baseline matrix; removed assets fail before final comparison.
pub(super) fn assert_protected_mutation(output: std::process::Output, mutation: &str) {
    assert!(!output.status.success(), "{mutation} mutation was accepted");
    let stage = if mutation == "asset removal" {
        "asset_id_codexy-marketplace-bundle.tar.gz"
    } else {
        "baseline_comparison"
    };
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&format!("stage={stage}")),
        "{mutation}: {output:?}"
    );
}
