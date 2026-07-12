mod support;

use support::{TestResult, copy_plugin_fixture, stderr, validator};

const CLAUSE: &str = "Status observation of a running packaged Sentinel MUST be read-only.";

fn validates_with(replacement: &str) -> TestResult<bool> {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(&skill_path, skill.replace(CLAUSE, replacement))?;
    let output = validator(&plugin_root, "--check")?;
    Ok(output.status.success() || !stderr(&output).contains("status observation"))
}

#[test]
fn rejects_numbered_false_metadata() -> TestResult {
    for prefix in ["Requirement 1: false —", "Requirement #1: false —"] {
        assert!(!validates_with(&format!("{prefix} {CLAUSE}"))?);
    }
    Ok(())
}

#[test]
fn rejects_additional_permissive_mutation_modals() -> TestResult {
    for continuation in [
        "It can interrupt a live Sentinel.",
        "The child may interrupt a live Sentinel.",
        "The child may poll a live Sentinel.",
        "The child may replace a live Sentinel.",
        "The parent could poll a live Sentinel.",
        "The root might send a status request.",
        "It may issue a follow-up prompt to a live Sentinel.",
        "The parent may replace a live Sentinel.",
        "The root may cancel a live Sentinel.",
        "The orchestrator might terminate a live Sentinel.",
        "The parent may not interrupt but may poll a live Sentinel.",
        "The parent may not interrupt but poll a live Sentinel.",
        "The parent may poll a live Sentinel, not interrupt it.",
        "The root policy owner may poll a live Sentinel.",
        "The parent may after a long wait interrupt a live Sentinel.",
        "The parent may message a live Sentinel.",
        "The child may be recorded after terminal completion. The parent may poll a live Sentinel.",
    ] {
        assert!(!validates_with(&format!("{CLAUSE} {continuation}"))?);
    }
    Ok(())
}

#[test]
fn allows_negated_or_non_mutating_additional_modals() -> TestResult {
    for continuation in [
        "It can be recorded after completion.",
        "The parent could not interrupt a live Sentinel.",
        "The root might never send a status request.",
        "After completion, the child may send the result to the parent.",
        "After completion, the child may poll a required CI check.",
        "After completion, the child may replace an archived ledger entry.",
    ] {
        assert!(validates_with(&format!("{CLAUSE} {continuation}"))?);
    }
    Ok(())
}

#[test]
fn ignores_later_child_completion_permissions() -> TestResult {
    assert!(validates_with(&format!(
        "{CLAUSE} It may be recorded after terminal completion. After completion, the child may send the result to the parent."
    ))?);
    Ok(())
}

#[test]
fn rejects_setext_heading_only_capture() -> TestResult {
    let replacement = "Sentinel observation policy\n---------------------------\n\nThe active policy is described elsewhere.";
    let heading = replacement.replace("Sentinel observation policy", CLAUSE);
    assert!(!validates_with(&heading)?);
    Ok(())
}

#[test]
fn rejects_fenced_code_only_capture() -> TestResult {
    for fenced in [
        format!("\n```markdown\n{CLAUSE}\n```"),
        format!("\n````markdown\n```\n{CLAUSE}\n````"),
        format!("\n~~~~markdown\n```\n{CLAUSE}\n~~~~"),
    ] {
        assert!(!validates_with(&fenced)?);
    }
    Ok(())
}

#[test]
fn indented_marker_does_not_hide_following_policy() -> TestResult {
    assert!(validates_with(&format!("\n    ```\n{CLAUSE}"))?);
    Ok(())
}
