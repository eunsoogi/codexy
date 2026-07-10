use std::process::{Command, Output};
mod support;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const BEGIN: &str = "# BEGIN CODEXY MANAGED AGENTS";
const END: &str = "# END CODEXY MANAGED AGENTS";
const MANAGED: &str = "# CODEXY MANAGED AGENT\n";
#[rustfmt::skip]
const AGENTS: [&str; 12] = ["codexy-architect", "codexy-tracer", "codexy-scribe", "codexy-cartographer", "codexy-forge", "codexy-weaver", "codexy-pathfinder", "codexy-auditor", "codexy-sculptor", "codexy-shipwright", "codexy-sentinel", "codexy-warden"];

#[cfg(unix)]
#[test]
fn registration_rejects_symlinked_discovery_root_without_outside_writes() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_parent = codex_home.join("agents");
    let outside = temp.path().join("outside-agents");
    std::fs::create_dir_all(&agents_parent)?;
    std::fs::create_dir(&outside)?;
    std::fs::write(outside.join("keep.txt"), "user-owned\n")?;
    symlink(&outside, agents_parent.join("codexy"))?;
    let before = directory_entries(&outside)?;

    let output = run(&plugin_root, &codex_home, &[])?;
    let after = directory_entries(&outside)?;

    assert!(
        !output.status.success()
            && after == before
            && std::fs::read_to_string(outside.join("keep.txt"))? == "user-owned\n",
        "registration must reject a symlinked discovery root without outside writes\nstatus: {}\nentries: {after:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(&output),
        stderr(&output)
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn registration_rejects_dangling_destination_symlink_without_partial_writes() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");
    let outside = temp.path().join("outside-sentinel.toml");
    let destination = agents_root.join("codexy-sentinel.toml");
    std::fs::create_dir_all(&agents_root)?;
    symlink(&outside, &destination)?;

    let output = run(&plugin_root, &codex_home, &[])?;
    let entries = directory_entries(&agents_root)?;
    let destination_is_symlink = std::fs::symlink_metadata(&destination)?
        .file_type()
        .is_symlink();

    assert!(
        !output.status.success()
            && destination_is_symlink
            && !outside.exists()
            && entries == ["codexy-sentinel.toml"],
        "registration must reject a dangling role symlink without replacing it or writing peers\nstatus: {}\nentries: {entries:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(&output),
        stderr(&output)
    );
    Ok(())
}

#[test]
fn migration_preserves_markers_inside_multiline_basic_and_literal_strings() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let config_path = codex_home.join("config.toml");
    std::fs::create_dir_all(&codex_home)?;
    std::fs::write(
        &config_path,
        r####"model = "gpt-5.5"
basic = """
basic before
# BEGIN CODEXY MANAGED AGENTS
basic payload
# END CODEXY MANAGED AGENTS
basic after
"""
literal = '''
literal before
# BEGIN CODEXY MANAGED AGENTS
literal payload
# END CODEXY MANAGED AGENTS
literal after
'''

# BEGIN CODEXY MANAGED AGENTS
[agents.codexy-sentinel]
config_file = "stale.toml"
# END CODEXY MANAGED AGENTS
"####,
    )?;

    let output = run(&plugin_root, &codex_home, &[])?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let config = std::fs::read_to_string(config_path)?;
    let basic = format!("basic before\n{BEGIN}\nbasic payload\n{END}\nbasic after");
    let literal = format!("literal before\n{BEGIN}\nliteral payload\n{END}\nliteral after");
    assert!(
        config.contains(&basic),
        "basic string was changed:\n{config}"
    );
    assert!(
        config.contains(&literal),
        "literal string was changed:\n{config}"
    );
    assert_eq!(config.matches(BEGIN).count(), 2, "config:\n{config}");
    assert_eq!(config.matches(END).count(), 2, "config:\n{config}");
    assert!(
        !config.contains("[agents.codexy-sentinel]"),
        "config:\n{config}"
    );
    Ok(())
}

#[test]
fn diagnose_rejects_twelve_marker_owned_files_with_stale_names() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");
    std::fs::create_dir_all(&agents_root)?;
    for index in 0..12 {
        let name = format!("codexy-retired-{index:02}");
        std::fs::write(
            agents_root.join(format!("{name}.toml")),
            format!(
                "{MANAGED}name = \"{name}\"\ndescription = \"Stale role\"\ndeveloper_instructions = \"Do stale work.\"\n"
            ),
        )?;
    }

    assert_diagnose_fails(&plugin_root, &codex_home)
}

#[test]
fn diagnose_rejects_wrong_marker_name_or_shape_in_expected_projections() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");
    write_expected_projections(&plugin_root, &agents_root)?;
    let source = std::fs::read_to_string(plugin_root.join("agents/codexy-sentinel.toml"))?;
    let expected = format!("{MANAGED}{source}");
    for invalid in [
        expected.replacen(MANAGED, "", 1),
        expected.replacen(
            "name = \"codexy-sentinel\"",
            "name = \"codexy-reviewer\"",
            1,
        ),
        format!(
            "{MANAGED}name = \"codexy-sentinel\"\ndescription = \"\"\ndeveloper_instructions = \"\"\n"
        ),
    ] {
        std::fs::write(agents_root.join("codexy-sentinel.toml"), invalid)?;
        assert_diagnose_fails(&plugin_root, &codex_home)?;
    }
    Ok(())
}

fn assert_diagnose_fails(
    plugin_root: &std::path::Path,
    codex_home: &std::path::Path,
) -> TestResult {
    let output = run(plugin_root, codex_home, &["--diagnose"])?;
    let stdout = stdout(&output);
    assert!(
        stdout.contains("A role-discovery: FAIL") && !stdout.contains("A role-discovery: PASS"),
        "semantic discovery errors must not pass by count\nstatus: {}\nstdout:\n{stdout}\nstderr:\n{}",
        output.status,
        stderr(&output)
    );
    Ok(())
}

fn write_expected_projections(
    plugin_root: &std::path::Path,
    agents_root: &std::path::Path,
) -> TestResult {
    std::fs::create_dir_all(agents_root)?;
    for name in AGENTS {
        let role = std::fs::read_to_string(plugin_root.join(format!("agents/{name}.toml")))?;
        std::fs::write(
            agents_root.join(format!("{name}.toml")),
            format!("{MANAGED}{role}"),
        )?;
    }
    Ok(())
}

fn installed_fixture(root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("installed-codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn run(
    plugin_root: &std::path::Path,
    codex_home: &std::path::Path,
    extra: &[&str],
) -> TestResult<Output> {
    Ok(
        Command::new(plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents"))
            .args([
                "--plugin-root",
                path(plugin_root)?,
                "--codex-home",
                path(codex_home)?,
            ])
            .args(extra)
            .output()?,
    )
}

fn directory_entries(root: &std::path::Path) -> std::io::Result<Vec<String>> {
    let mut entries = std::fs::read_dir(root)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn path(path: &std::path::Path) -> TestResult<&str> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
