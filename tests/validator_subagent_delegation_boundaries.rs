use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_line_wrapped_recursive_permissions() -> TestResult {
    for permission in [
        "Allowed actions:\n- spawning another helper for QA.",
        "Allowed actions:\n1. Spawning another helper for QA.",
        "A helper MAY\nspawn another helper.",
        "A helper is allowed to spawn\nanother helper.",
    ] {
        assert_recursive_role_permission_rejected(permission)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_allowed_action_bullets_and_circumstantial_must() -> TestResult {
    assert_recursive_role_permission_rejected(
        "Allowed actions:\n- Map files.\n- Spawn another helper.",
    )?;
    assert_role_recursion_not_reported(
        "A helper MUST, under no circumstances, spawn another helper.",
    )?;
    assert_role_recursion_not_reported("A helper MUST never spawn another helper.")?;
    Ok(())
}

#[test]
fn validator_rejects_recursive_actions_after_unrelated_negations() -> TestResult {
    for permission in [
        "MUST spawn validator_edge_pass and workflow_ownership_pass as additional helpers.",
        "MUST create a child thread.",
        "MUST immediately spawn another helper.",
        "MUST spawn another Sentinel.",
        "MUST delegate work to another specialist.",
        "Allowed actions: creating reviewer tasks.",
        "Allowed actions: delegating work to helper threads.",
        "MUST NOT merge, but MAY spawn another helper.",
        "A helper MAY not edit files, but MAY spawn another helper.",
        "Allowed actions: MUST NOT edit files, but spawn another helper.",
        "Permitted actions: MUST NOT merge, but create reviewer tasks.",
        "A helper is not allowed to merge, but is allowed to spawn another helper.",
    ] {
        assert_recursive_role_permission_rejected(permission)?;
    }
    Ok(())
}

#[test]
fn validator_does_not_flag_punctuated_nonrecursive_prohibitions() -> TestResult {
    for prohibition in [
        "A helper MAY, under no circumstances, spawn another helper.",
        "A helper MAY not, under any circumstances, spawn another helper.",
        "A helper MAY never, even during recovery, create another reviewer task.",
        "A helper MAY edit files but not spawn another helper.",
        "A helper MAY edit files but never spawn another helper.",
        "A helper is not allowed, under this contract, to delegate work to another reviewer.",
        "Allowed actions: map files, but MUST NOT spawn another helper.",
        "Every helper MUST NOT spawn, delegate to, or create any additional agent.",
    ] {
        assert_role_recursion_not_reported(prohibition)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_qualified_and_extended_recursive_actions() -> TestResult {
    for permission in [
        "A helper is allowed, after owner approval, to spawn another helper.",
        "A helper is permitted, after owner approval, to spawn another helper.",
        "A helper MAY spawn another worker.",
        "A helper MAY delegate work to another explorer.",
        "A helper MAY spawn another subagent.",
        "A helper MAY start another subagent.",
        "A helper MAY fork a child thread.",
    ] {
        assert_recursive_role_permission_rejected(permission)?;
    }
    Ok(())
}

#[test]
fn validator_allows_non_delegating_derived_words() -> TestResult {
    for instruction in [
        "A helper MAY review delegated tasks.",
        "A helper MAY inspect a created thread id.",
        "A helper MAY start a local analysis.",
    ] {
        assert_role_recursion_not_reported(instruction)?;
    }
    Ok(())
}

#[test]
fn validator_resets_allowed_actions_context_at_boundaries() -> TestResult {
    for instruction in [
        "Allowed actions:\n- Map files.\n\n- Spawn another helper.",
        "Allowed actions:\n- Map files.\nForbidden actions:\n- Spawn another helper.",
        "Allowed actions:\n- Map files.\n## Forbidden actions\n- Spawn another helper.",
    ] {
        assert_role_recursion_not_reported(instruction)?;
    }
    Ok(())
}

fn assert_recursive_role_permission_rejected(permission: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let role_path = plugin_root.join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replacen("\n\"\"\"", &format!("\n{permission}\n\"\"\""), 1),
    )?;
    assert_recursion_rejected(validator(&plugin_root)?, permission);
    Ok(())
}

fn assert_role_recursion_not_reported(instruction: &str) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = fixture(&temp)?;
    let role_path = plugin_root.join("agents/codexy-cartographer.toml");
    let role = std::fs::read_to_string(&role_path)?;
    std::fs::write(
        &role_path,
        role.replacen("\n\"\"\"", &format!("\n{instruction}\n\"\"\""), 1),
    )?;
    assert!(!stderr(&validator(&plugin_root)?).contains("permits recursive delegation"));
    Ok(())
}

fn fixture(temp: &tempfile::TempDir) -> TestResult<std::path::PathBuf> {
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
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

fn assert_recursion_rejected(output: Output, example: &str) {
    assert!(!output.status.success(), "{example}");
    assert!(
        stderr(&output).contains("permits recursive delegation"),
        "{example}"
    );
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
