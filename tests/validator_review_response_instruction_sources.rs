use crate::support::{copy_plugin_fixture, stderr, TestResult};

const SENTINEL_PATH: &str = "agents/codexy-sentinel.toml";
const SENTINEL_CLAUSE: &str =
    "Sentinel MUST consolidate examples from the same defect class into one blocker with one structural repair strategy.";
const ORCHESTRATION_PATH: &str = "skills/codex-orchestration/SKILL.md";
const ORCHESTRATION_CLAUSE: &str =
    "Before review-response edits, MUST create one root-cause cluster for each actionable defect class.";

#[test]
fn active_review_cluster_contract_sources_pass() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let output = crate::support::validator_instruction_policy(&plugin_root)?;
    assert!(output.status.success(), "unexpected failure: {}", stderr(&output));
    Ok(())
}

#[test]
fn toml_comments_cannot_satisfy_review_cluster_contracts() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(SENTINEL_PATH);
    let text = std::fs::read_to_string(&path)?;
    let replaced = text.replacen(SENTINEL_CLAUSE, "Removed active Sentinel contract.", 1);
    let commented = format!("{replaced}\n# {SENTINEL_CLAUSE}\n");
    toml::from_str::<toml::Value>(&commented)?;
    std::fs::write(path, commented)?;

    assert_contract_rejected(&plugin_root)
}

#[test]
fn inactive_markdown_cannot_satisfy_review_cluster_contracts() -> TestResult {
    for inactive in [
        format!("<!-- {ORCHESTRATION_CLAUSE} -->"),
        format!("```text\n{ORCHESTRATION_CLAUSE}\n```"),
        format!("    {ORCHESTRATION_CLAUSE}"),
        format!("<pre>\n{ORCHESTRATION_CLAUSE}\n</pre>"),
        format!("<code class=\"example\">\n{ORCHESTRATION_CLAUSE}\n</code>"),
        format!("<SCRIPT type=\"text/plain\">{ORCHESTRATION_CLAUSE}</SCRIPT>"),
        format!("<pre>\n</prefix>\n{ORCHESTRATION_CLAUSE}\n</pre>"),
        format!("<pre\n class=\"example\">\n{ORCHESTRATION_CLAUSE}\n</pre>"),
        format!("<template>\n{ORCHESTRATION_CLAUSE}\n</template>"),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let path = plugin_root.join(ORCHESTRATION_PATH);
        let text = std::fs::read_to_string(&path)?;
        let replaced = text.replacen(
            ORCHESTRATION_CLAUSE,
            "Removed active orchestration contract.",
            1,
        );
        std::fs::write(path, format!("{replaced}\n{inactive}\n"))?;

        assert_contract_rejected(&plugin_root)?;
    }
    Ok(())
}

#[test]
fn inline_code_html_tag_examples_do_not_hide_active_contracts() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(ORCHESTRATION_PATH);
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        text.replacen(
            ORCHESTRATION_CLAUSE,
            &format!("Inline example: `<pre>`.\n{ORCHESTRATION_CLAUSE}"),
            1,
        ),
    )?;

    let output = crate::support::validator_instruction_policy(&plugin_root)?;
    assert!(output.status.success(), "unexpected failure: {}", stderr(&output));
    Ok(())
}

#[test]
fn escaped_html_tag_examples_do_not_hide_active_contracts() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(ORCHESTRATION_PATH);
    let text = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        text.replacen(
            ORCHESTRATION_CLAUSE,
            &format!("Escaped example: \\<pre>.\n{ORCHESTRATION_CLAUSE}"),
            1,
        ),
    )?;

    let output = crate::support::validator_instruction_policy(&plugin_root)?;
    assert!(output.status.success(), "unexpected failure: {}", stderr(&output));
    Ok(())
}

#[test]
fn fenced_toml_prompt_examples_cannot_satisfy_review_cluster_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(SENTINEL_PATH);
    let text = std::fs::read_to_string(&path)?;
    let fenced = text.replacen(
        SENTINEL_CLAUSE,
        &format!("Removed active Sentinel contract.\n```text\n{SENTINEL_CLAUSE}\n```"),
        1,
    );
    toml::from_str::<toml::Value>(&fenced)?;
    std::fs::write(path, fenced)?;

    assert_contract_rejected(&plugin_root)
}

#[test]
fn negated_markdown_clause_cannot_satisfy_review_cluster_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(ORCHESTRATION_PATH);
    let text = std::fs::read_to_string(&path)?;
    let negated = text.replacen(
        ORCHESTRATION_CLAUSE,
        &format!("It is false that {ORCHESTRATION_CLAUSE}"),
        1,
    );
    std::fs::write(path, negated)?;

    assert_contract_rejected(&plugin_root)
}

#[test]
fn trailing_disclaimer_cannot_satisfy_review_cluster_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let path = plugin_root.join(ORCHESTRATION_PATH);
    let text = std::fs::read_to_string(&path)?;
    let disclaimed = text.replacen(
        ORCHESTRATION_CLAUSE,
        &format!(
            "{ORCHESTRATION_CLAUSE} This requirement is optional and need not be followed."
        ),
        1,
    );
    std::fs::write(path, disclaimed)?;

    assert_contract_rejected(&plugin_root)
}

fn assert_contract_rejected(plugin_root: &std::path::Path) -> TestResult {
    let output = crate::support::validator_instruction_policy(plugin_root)?;
    assert!(!output.status.success(), "inactive contract unexpectedly passed");
    assert!(
        stderr(&output).contains("root-cause review cluster"),
        "unexpected diagnostic: {}",
        stderr(&output)
    );
    Ok(())
}
