use crate::support;
use crate::support::FixtureCommand as Command;
use std::process::Output;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn assert_diagnose_fails(
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

pub(super) fn directory_entries(root: &std::path::Path) -> std::io::Result<Vec<String>> {
    let mut entries = std::fs::read_dir(root)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

pub(super) fn installed_fixture(root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("installed-codexy");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

pub(super) fn run(
    plugin_root: &std::path::Path,
    codex_home: &std::path::Path,
    extra: &[&str],
) -> TestResult<Output> {
    Ok(
        Command::new(plugin_root.join("skills/orchestration/scripts/register-codexy-agents.py"))
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

pub(super) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub(super) fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn path(path: &std::path::Path) -> TestResult<&str> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}
