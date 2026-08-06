type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_requires_typed_affirmation_and_exact_authoritative_owner() -> TestResult {
    let owners = [
        "current-thread-owned",
        "child-owned",
        "parent-owned",
        "external/human-owned",
    ];
    for authority in owners {
        for decision_owner in owners {
            for affirmation in ["affirmative", "denied"] {
                let decision = format!(
                    "{affirmation} {decision_owner} because opaque rationale"
                );
                let expected = affirmation == "affirmative" && authority == decision_owner;
                assert_control(authority, &decision, expected)?;
            }
        }
    }
    for authority in ["unknown", "child-owned or parent-owned"] {
        for affirmation in ["affirmative", "denied"] {
            assert_control(
                authority,
                &format!("{affirmation} child-owned because opaque rationale"),
                false,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_applies_one_typed_rationale_grammar_to_every_owner() -> TestResult {
    assert_control("parent-owned", "parent-owned not allowed to act", false)?;
    for owner in [
        "current-thread-owned",
        "child-owned",
        "parent-owned",
        "external/human-owned",
    ] {
        for (decision, expected) in [
            (format!("affirmative {owner}"), true),
            (format!("affirmative {owner} because"), false),
            (
                format!("affirmative {owner} because opaque rationale"),
                true,
            ),
            (format!("{owner} implementation lane"), false),
        ] {
            assert_control(owner, &decision, expected)?;
        }
    }
    Ok(())
}

fn assert_control(authority: &str, decision: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(
        &path,
        format!(
            "{}\nPlan tool call: update_plan\n",
            classification(authority, decision)
        ),
    )?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "authority={authority}; decision={decision}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn classification(authority: &str, decision: &str) -> String {
    format!(
        "Ownership metadata source: current-thread-classified\nLane ownership: {authority}\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | validation |\n| Secondary surfaces | validators |\n| Owner decision | {decision} |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | validate after classification |\n| Stop/blocker | None |"
    )
}
