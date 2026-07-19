use std::path::Path;
use std::process::{Command, Output};

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_accepts_mandatory_reasoning_evidence_omission_prohibitions() -> TestResult {
    for replacement in [
        "Every approval MUST NOT omit reasoning control used or unavailable evidence",
        "Every approval MUST NOT skip reasoning control used or unavailable evidence",
    ] {
        let output = validate_sentinel_edit(|sentinel| {
            Ok(sentinel.replace(
                "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
                &format!(
                    "and any unresolved risk. {replacement}. MUST identify formatting-only LOC remediation before approving readiness."
                ),
            ))
        })?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_mandatory_omission_prohibitions_with_later_evidence() -> TestResult {
    for prohibition in ["omit", "skip", "leave out"] {
        let output = validate_sentinel_edit(|sentinel| {
            Ok(sentinel.replace(
                "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
                &format!(
                    "and any unresolved risk. Every approval MUST NOT {prohibition} reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed. MUST identify formatting-only LOC remediation before approving readiness."
                ),
            ))
        })?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_mandatory_omission_prohibition_with_later_evidence_markers() -> TestResult
{
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel.replace(
            "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
            "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
        ))
    })?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_waiver_after_affirmative_evidence_list() -> TestResult {
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel.replace(
            "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
            "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed. This evidence may be omitted.. MUST identify formatting-only LOC remediation before approving readiness.",
        ))
    })?;
    assert!(!output.status.success(), "accepted cross-sentence waiver");
    assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_followups_after_affirmative_evidence_list() -> TestResult {
    for followup in [
        "This evidence cannot be included.",
        "This evidence may not be included.",
    ] {
        let output = validate_sentinel_edit(|sentinel| {
            Ok(sentinel.replace(
                "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
                &format!(
                    "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed. {followup}. MUST identify formatting-only LOC remediation before approving readiness."
                ),
            ))
        })?;
        assert!(!output.status.success(), "accepted {followup:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_passive_mandatory_followup_after_affirmative_evidence_list() -> TestResult
{
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel.replace(
            "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
            "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed. This evidence MUST NOT be omitted. MUST identify formatting-only LOC remediation before approving readiness.",
        ))
    })?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_permissive_suffix_after_affirmative_evidence_list() -> TestResult {
    for suffix in [
        "but this evidence can be absent",
        "but this evidence may be disregarded",
    ] {
        let output = validate_sentinel_edit(|sentinel| {
            Ok(sentinel.replace(
                "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
                &format!(
                    "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed, {suffix}.. MUST identify formatting-only LOC remediation before approving readiness."
                ),
            ))
        })?;
        assert!(!output.status.success(), "accepted {suffix:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_undelimited_permissive_suffix_after_affirmative_evidence_list()
-> TestResult {
    for suffix in [
        "but this evidence can be absent",
        "but this evidence may be disregarded",
    ] {
        let output = validate_sentinel_edit(|sentinel| {
            Ok(sentinel.replace(
                "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
                &format!(
                    "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed {suffix}.. MUST identify formatting-only LOC remediation before approving readiness."
                ),
            ))
        })?;
        assert!(!output.status.success(), "accepted {suffix:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}
fn validate_sentinel_edit(edit: impl FnOnce(String) -> TestResult<String>) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel)?)?;
    validator(&plugin_root)
}

fn copy_fixture(plugin_root: &Path) -> std::io::Result<()> {
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
