use std::{fs, path::Path, process::Command};

use crate::support::copy_dir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn direct_devtools_check_rejects_malformed_skill_frontmatter() -> TestResult {
    let temp = tempfile::tempdir()?;
    let devtools = temp.path().join("plugins/codexy-devtools");
    copy_devtools(&devtools)?;
    let skill = devtools.join("skills/developer-tools/SKILL.md");
    fs::write(&skill, fs::read_to_string(&skill)?.replacen("name: developer-tools", "name: 'developer-tools", 1))?;

    assert_rejected(&devtools, "frontmatter must be valid YAML")
}

#[test]
fn direct_devtools_check_rejects_invalid_agent_metadata() -> TestResult {
    for relative in [
        "agents/openai.yaml",
        "skills/developer-tools/agents/openai.yaml",
    ] {
        let temp = tempfile::tempdir()?;
        let devtools = temp.path().join("plugins/codexy-devtools");
        copy_devtools(&devtools)?;
        let metadata = devtools.join(relative);
        fs::write(
            &metadata,
            fs::read_to_string(&metadata)?.replace(
                "allow_implicit_invocation: true",
                "allow_implicit_invocation: false",
            ),
        )?;

        assert_rejected(&devtools, "policy.allow_implicit_invocation must be true")?;
    }
    Ok(())
}

#[test]
fn aggregate_core_check_rejects_devtools_skill_and_agent_metadata() -> TestResult {
    for (relative, replacement, expected) in [
        (
            "skills/developer-tools/SKILL.md",
            "name: 'developer-tools",
            "frontmatter must be valid YAML",
        ),
        (
            "agents/openai.yaml",
            "\tdisplay_name:",
            "must not contain tab indentation",
        ),
    ] {
        let temp = tempfile::tempdir()?;
        let core = temp.path().join("plugins/codexy");
        let devtools = temp.path().join("plugins/codexy-devtools");
        copy_core(&core)?;
        copy_devtools(&devtools)?;
        let target = devtools.join(relative);
        let source = fs::read_to_string(&target)?;
        let mutated = if relative.ends_with("SKILL.md") {
            source.replacen("name: developer-tools", replacement, 1)
        } else {
            source.replacen("  display_name:", replacement, 1)
        };
        fs::write(target, mutated)?;

        assert_rejected(&core, expected)?;
    }
    Ok(())
}

fn copy_core(target: &Path) -> std::io::Result<()> {
    copy_dir(codexy_runtime::paths::repository_root().join("plugins/codexy"), target)
}

fn copy_devtools(target: &Path) -> std::io::Result<()> {
    copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy-devtools"),
        target,
    )
}

fn assert_rejected(plugin_root: &Path, expected: &str) -> TestResult {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", plugin_root.to_str().ok_or("plugin root")?, "--check"])
        .output()?;
    assert!(!output.status.success(), "validator unexpectedly accepted fixture");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
