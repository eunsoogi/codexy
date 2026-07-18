use std::path::Path;
use std::process::{Command, Output};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

pub(crate) fn fixture(
    path: &str,
    source: String,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    run(repo.path(), &["init", "-q"])?;
    if ["src/bin/", "tests/", "examples/", "benches/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        write(repo.path(), "Cargo.toml", "[package]\nname = \"app\"\n")?;
    }
    if let Some(target) = match path {
        "src/custom_bin.rs" => Some("src/custom_bin.rs"),
        "src/custom_dot_bin.rs" => Some("./src//./custom_dot_bin.rs"),
        "src/custom_parent_bin.rs" => Some("src/generated/../custom_parent_bin.rs"),
        "src/custom_escape.rs" => Some("../src/custom_escape.rs"),
        _ => None,
    } {
        write(
            repo.path(),
            "Cargo.toml",
            &format!(
                "[package]\nname = \"app\"\n[[bin]]\nname = \"custom\"\npath = \"{target}\"\n"
            ),
        )?;
    }
    if path.starts_with("crates/app/") {
        write(
            repo.path(),
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\n",
        )?;
    }
    write(repo.path(), path, &source)?;
    run(repo.path(), &["add", "."])?;
    run(
        repo.path(),
        &[
            "-c",
            "user.email=codexy@example.test",
            "-c",
            "user.name=Codexy Test",
            "commit",
            "-qm",
            "initial",
        ],
    )?;
    Ok(repo)
}

pub(crate) fn write(root: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture file parent"))?;
    std::fs::write(path, text)
}

pub(crate) fn validate(root: &Path) -> Result<Output, Box<dyn std::error::Error>> {
    let diagnostics = codexy_runtime::validation::touched_loc_diagnostics(root, "HEAD")?;
    let success = diagnostics.is_empty();
    let stderr = if success {
        String::new()
    } else {
        diagnostics
            .iter()
            .map(|diagnostic| format!("error: {diagnostic}"))
            .chain(std::iter::once(format!(
                "Error: touched LOC validation failed with {} error(s)",
                diagnostics.len()
            )))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };
    Ok(Output {
        status: exit_status(success),
        stdout: success
            .then_some("plugin config validation ok: plugins/codexy\n")
            .unwrap_or_default()
            .as_bytes()
            .to_vec(),
        stderr: stderr.into_bytes(),
    })
}

pub(crate) fn regular_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

pub(crate) fn regular_lines_from(start: usize, count: usize) -> String {
    (start..start + count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn run(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        stderr(&output)
    );
    Ok(())
}

fn exit_status(success: bool) -> std::process::ExitStatus {
    #[cfg(unix)]
    return std::process::ExitStatus::from_raw(if success { 0 } else { 1 << 8 });
    #[cfg(windows)]
    std::process::ExitStatus::from_raw(i32::from(!success))
}
