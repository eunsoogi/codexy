use std::{fs, process::Command};

use super::super::workflow_profile_contract::{assert_profile_result, formal_classification};
use super::support::{
    MATERIAL_RISK_TRIGGERS, TestResult, classification_request, expected_reviewer, resolve_profile,
};
use serde_json::Value;

#[test]
fn eligible_light_and_standard_routes_ignore_legacy_collaboration_flags() -> TestResult {
    let collaboration = [
        ("none", Vec::new()),
        ("delegation", vec![("durable_delegation", true)]),
        ("independent lanes", vec![("multi_lane_ownership", true)]),
        (
            "both flags",
            vec![("durable_delegation", true), ("multi_lane_ownership", true)],
        ),
        (
            "explicit false flags",
            vec![("durable_delegation", false), ("multi_lane_ownership", false)],
        ),
    ];
    for (work_class, eligible, expected) in
        [("low_risk", true, "light"), ("middle", false, "standard")]
    {
        for (description, flags) in &collaboration {
            let route = resolve_profile(&classification_request(
                work_class,
                eligible,
                &[],
                flags,
                None,
            ))?;
            assert_eq!(
                route["profile"], expected,
                "{work_class} work changed profile because of {description}"
            );
            assert_eq!(route["reviewer"], expected_reviewer(expected));
        }
    }
    Ok(())
}

#[test]
fn every_material_risk_and_explicit_requirement_selects_strict() -> TestResult {
    for trigger in MATERIAL_RISK_TRIGGERS {
        let route = resolve_profile(&classification_request(
            "middle",
            false,
            &[trigger],
            &[],
            None,
        ))?;
        assert_eq!(route["profile"], "strict", "{trigger} must select strict");
        assert_eq!(route["reviewer"], expected_reviewer("strict"));
    }
    Ok(())
}

#[test]
fn structured_and_markdown_routes_agree_on_collaboration_and_shared_contract_risk() -> TestResult {
    let collaboration = resolve_profile(&classification_request(
        "middle",
        false,
        &[],
        &[("multi_lane_ownership", true)],
        None,
    ))?;
    assert_eq!(collaboration["profile"], "standard");
    let middle_text = format!(
        "Workflow profile: standard\nTask kind: ordinary single-owner mutation\nMulti-lane ownership: yes\n{}",
        formal_classification()
    );
    assert_profile_result(
        "independent lanes keep the equivalent standard Markdown route",
        &middle_text,
        true,
    )?;

    let shared_contract = resolve_profile(&classification_request(
        "middle",
        false,
        &["materially_shared_executable_contract"],
        &[],
        None,
    ))?;
    assert_eq!(shared_contract["profile"], "strict");
    for (risk_text, description) in [
        (
            "materially shared executable-contract change",
            "singular shared executable-contract risk",
        ),
        (
            "materially shared executable contracts",
            "plural shared executable-contract risk",
        ),
    ] {
        for requested in ["light", "standard"] {
            let text = format!(
                "Workflow profile: {requested}\nTask kind: {risk_text}\n{}",
                formal_classification()
            );
            assert_profile_result(
                &format!("a weaker Markdown choice cannot override {description}"),
                &text,
                false,
            )?;
        }
    }

    let negated = format!(
        "Workflow profile: standard\nTask kind: no shared executable-contract change\n{}",
        formal_classification()
    );
    assert_profile_result(
        "local negation preserves the equivalent standard route",
        &negated,
        true,
    )?;
    let not_shared = format!(
        "Workflow profile: standard\nTask kind: not a materially shared executable-contract change\n{}",
        formal_classification()
    );
    assert_profile_result(
        "article-aware negation preserves the equivalent standard route",
        &not_shared,
        true,
    )?;
    let quoted = format!(
        "Workflow profile: standard\nTask kind: ordinary single-owner mutation\nQuoted example: `materially shared executable contracts`\n{}",
        formal_classification()
    );
    assert_profile_result(
        "an inline-code example does not add shared-contract risk",
        &quoted,
        true,
    )
}

#[test]
fn strict_risk_escalates_a_lower_prior_profile_without_changing_reviewer_identity() -> TestResult {
    for prior in ["light", "standard"] {
        let route = resolve_profile(&classification_request(
            "low_risk",
            true,
            &["explicit_strict_requirement"],
            &[],
            Some(prior),
        ))?;
        assert_eq!(route["profile"], "strict");
        assert_eq!(route["discarded_lower_profile"], prior);
        assert_eq!(route["reviewer"], expected_reviewer("strict"));
    }

    let route = resolve_profile(&classification_request("low_risk", true, &[], &[], None))?;
    assert_eq!(route["profile"], "light");
    assert_eq!(route["reviewer"], Value::Null);

    let route = resolve_profile(&classification_request("middle", false, &[], &[], None))?;
    assert_eq!(route["profile"], "standard");
    assert_eq!(route["reviewer"], expected_reviewer("standard"));
    Ok(())
}

#[test]
fn command_line_resolver_uses_the_same_material_risk_contract() -> TestResult {
    for (risks, collaboration, expected) in [
        (
            Vec::<&str>::new(),
            vec![("durable_delegation", true), ("multi_lane_ownership", true)],
            "light",
        ),
        (
            vec!["materially_shared_executable_contract"],
            vec![("multi_lane_ownership", true)],
            "strict",
        ),
    ] {
        let temporary = tempfile::tempdir()?;
        let input = temporary.path().join("classification.json");
        let request = classification_request(
            "low_risk",
            true,
            &risks,
            &collaboration,
            None,
        );
        fs::write(&input, serde_json::to_vec(&request)?)?;
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
            .args(["--resolve-profile", "--input"])
            .arg(&input)
            .output()?;
        assert!(
            output.status.success(),
            "resolver rejected the request: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let route: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(route["profile"], expected);
        assert_eq!(route["reviewer"], expected_reviewer(expected));
    }
    Ok(())
}
