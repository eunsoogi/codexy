use serde_json::{Value, json};

use crate::support::TestResult;

use super::resolve_profile;

#[test]
fn review_profile_requires_a_typed_validated_classification() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    for profile in ["light", "standard", "strict"] {
        let output = resolve_profile(fixture.root(), classified(profile, &[]))?;
        assert!(
            output.status.success(),
            "typed {profile} classification must resolve: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let route: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(route["profile"], profile);
    }
    assert!(
        !resolve_profile(
            fixture.root(),
            json!({"schema":"codexy.review-profile-request.v1","profile":"light"})
        )?
        .status
        .success(),
        "a bare caller profile must not select a review route"
    );
    Ok(())
}

#[test]
fn strict_triggered_classification_cannot_downgrade_to_light_or_standard() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    for profile in ["light", "standard"] {
        let output = resolve_profile(fixture.root(), classified(profile, &["durable_delegation"]))?;
        assert!(
            !output.status.success(),
            "strict trigger must reject a {profile} route: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(
        resolve_profile(fixture.root(), classified("strict", &["durable_delegation"]))?
            .status
            .success()
    );
    Ok(())
}

fn classified(profile: &str, strict_triggers: &[&str]) -> Value {
    json!({
        "schema":"codexy.review-profile-request.v1",
        "classification": {
            "schema":"codexy.workflow-profile-classification.v1",
            "profile":profile,
            "strict_triggers":strict_triggers
        }
    })
}
