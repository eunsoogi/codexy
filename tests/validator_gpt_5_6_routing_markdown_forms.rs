use crate::support;

use support::routing_validator::{
    TestResult, assert_accepted, assert_policy_rejected, duplicate_recipient_section,
};

#[test]
fn validator_checks_routes_in_active_nested_lists() -> TestResult {
    let nested = |model| {
        duplicate_recipient_section(&format!(
            "- Routes:\n    - child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };

    assert_policy_rejected(nested("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(nested("gpt-5.6-sol")?)?;
    assert_accepted(duplicate_recipient_section(
        "- Routes:\n    - child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n      and `thinking: \"high\"`.",
    )?)?;
    assert_policy_rejected(
        duplicate_recipient_section(
            "- Routes:\n    - child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"`\n      and `thinking: \"high\"`.\n    - child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
        )?,
        "gpt-5.6-sol/high",
    )?;
    assert_accepted(duplicate_recipient_section(
        "    - child-to-root delivery MUST pass `model: \"gpt-5.6-terra\"` and `thinking: \"high\"`.",
    )?)
}

#[test]
fn validator_matches_hash_closed_recipient_headings() -> TestResult {
    let section = |heading: &str, model: &str| {
        duplicate_recipient_section(&format!(
            "{heading}\n\n- child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };

    assert_policy_rejected(
        section("## Recipient Model Routing ##", "gpt-5.6-terra")?,
        "gpt-5.6-sol/high",
    )?;
    assert_accepted(section("## Recipient Model Routing ##", "gpt-5.6-sol")?)?;
    assert_accepted(section("## Recipient Model Routing##", "gpt-5.6-terra")?)
}

#[test]
fn validator_keeps_nested_lists_active_across_blank_lines() -> TestResult {
    let nested = |model| {
        duplicate_recipient_section(&format!(
            "- Routes:\n\n    - child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };
    assert_policy_rejected(nested("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(nested("gpt-5.6-sol")?)
}

#[test]
fn validator_checks_wide_nested_continuations() -> TestResult {
    let nested = |suffix| {
        duplicate_recipient_section(&format!(
            "- Routes:\n    - child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`\n        {suffix}"
        ))
    };
    assert_policy_rejected(
        nested("and MUST pass `model: \"gpt-5.6-terra\"`.")?,
        "gpt-5.6-sol/high",
    )?;
    assert_accepted(nested("and MUST preserve that assignment.")?)
}

#[test]
fn validator_matches_tab_delimited_closing_hashes() -> TestResult {
    let section = |model| {
        duplicate_recipient_section(&format!(
            "## Recipient Model Routing\t##\n\n- child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };
    assert_policy_rejected(section("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(section("gpt-5.6-sol")?)
}

#[test]
fn validator_checks_fields_with_whitespace_before_colons() -> TestResult {
    let section = |model| {
        duplicate_recipient_section(&format!(
            "- child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`, then `model : \"{model}\"` and `thinking : \"high\"`."
        ))
    };
    assert_policy_rejected(section("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(section("gpt-5.6-sol")?)
}

#[test]
fn validator_matches_tab_delimited_opening_hashes() -> TestResult {
    let section = |model| {
        duplicate_recipient_section(&format!(
            "## Other Policy\n\n##\tRecipient Model Routing\n\n- child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };
    assert_policy_rejected(section("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(section("gpt-5.6-sol")?)
}

#[test]
fn validator_checks_tab_delimited_list_items() -> TestResult {
    let section = |model| {
        duplicate_recipient_section(&format!(
            "-\tchild-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };
    assert_policy_rejected(section("gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
    assert_accepted(section("gpt-5.6-sol")?)
}

#[test]
fn validator_checks_commonmark_lazy_continuations() -> TestResult {
    assert_policy_rejected(
        duplicate_recipient_section(
            "- child-to-root delivery MUST\nNOT pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.",
        )?,
        "gpt-5.6-sol/high",
    )?;
    assert_accepted(duplicate_recipient_section(
        "- child-to-root delivery MUST pass `model: \"gpt-5.6-sol\"` and `thinking: \"high\"`.\nThis sentence preserves the assignment.",
    )?)
}

#[test]
fn validator_checks_active_task_list_items() -> TestResult {
    let item = |prefix: &str, model: &str| {
        duplicate_recipient_section(&format!(
            "-{prefix}child-to-root delivery MUST pass `model: \"{model}\"` and `thinking: \"high\"`."
        ))
    };
    for prefix in [" [x] ", " [ ] ", " [x]\t", "  [x] ", " [\t] "] {
        assert_policy_rejected(item(prefix, "gpt-5.6-terra")?, "gpt-5.6-sol/high")?;
        assert_accepted(item(prefix, "gpt-5.6-sol")?)?;
    }
    Ok(())
}
