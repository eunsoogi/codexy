use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

struct GitFixtureSeed {
    _temporary: tempfile::TempDir,
    metadata: std::path::PathBuf,
}

static GIT_FIXTURE_SEEDS: OnceLock<Mutex<HashMap<Vec<(String, String)>, GitFixtureSeed>>> =
    OnceLock::new();

pub(crate) fn fixture(
    path: &str,
    source: String,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let mut initial_files = Vec::new();
    if ["src/bin/", "tests/", "examples/", "benches/"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        set_initial_file(
            &mut initial_files,
            "Cargo.toml",
            "[package]\nname = \"app\"\n",
        );
    }
    if let Some(target) = match path {
        "src/custom_bin.rs" => Some("src/custom_bin.rs"),
        "src/custom_dot_bin.rs" => Some("./src//./custom_dot_bin.rs"),
        "src/custom_parent_bin.rs" => Some("src/generated/../custom_parent_bin.rs"),
        "src/custom_escape.rs" => Some("../src/custom_escape.rs"),
        _ => None,
    } {
        set_initial_file(
            &mut initial_files,
            "Cargo.toml",
            &format!(
                "[package]\nname = \"app\"\n[[bin]]\nname = \"custom\"\npath = \"{target}\"\n"
            ),
        );
    }
    if path.starts_with("crates/app/") {
        set_initial_file(
            &mut initial_files,
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\n",
        );
    }
    set_initial_file(&mut initial_files, path, &source);
    let seed = git_fixture_seed(&initial_files)?;
    let repo = tempfile::tempdir()?;
    super::copy_dir(&seed, &repo.path().join(".git"))?;
    for (path, source) in &initial_files {
        write(repo.path(), path, source)?;
    }
    Ok(repo)
}

fn set_initial_file(files: &mut Vec<(String, String)>, path: &str, source: &str) {
    if let Some((_, existing)) = files.iter_mut().find(|(existing, _)| existing == path) {
        *existing = source.to_owned();
    } else {
        files.push((path.to_owned(), source.to_owned()));
    }
}

fn git_fixture_seed(
    files: &[(String, String)],
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let key = files.to_vec();
    let seeds = GIT_FIXTURE_SEEDS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut seed = seeds
        .lock()
        .map_err(|_| std::io::Error::other("touched-LOC Git fixture seed lock"))?;
    if let Some(existing) = seed.get(&key) {
        return Ok(existing.metadata.clone());
    }
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    std::fs::create_dir_all(&root)?;
    for (path, source) in files {
        write(&root, path, source)?;
    }
    // Later mutation cases amend their initial commit. Keep every immutable
    // seed private so one fixture can never affect a sibling fixture.
    run(&root, &["init", "-q"])?;
    // Git maintenance can mutate .git/objects while a cached seed is copied.
    // Disable it before creating the snapshot that later fixtures share.
    run(&root, &["config", "maintenance.auto", "false"])?;
    run(&root, &["config", "user.email", "codexy@example.test"])?;
    run(&root, &["config", "user.name", "Codexy Test"])?;
    run(&root, &["add", "."])?;
    run(&root, &["commit", "-qm", "initial"])?;
    let metadata = root.join(".git");
    seed.insert(
        key,
        GitFixtureSeed {
            metadata: metadata.clone(),
            _temporary: temporary,
        },
    );
    Ok(metadata)
}

pub(crate) fn write(root: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture file parent"))?;
    std::fs::write(&path, text).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("fixture write: path={}: {error}", path.display()),
        )
    })
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

pub(crate) fn run_cargo(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    // Keep package/toolchain caches inherited while isolating build artifacts
    // for each temporary fixture root.
    Command::new("cargo")
        .args(args)
        .current_dir(root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .output()
}

fn run(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    super::profile_metrics::record("git_command");
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("git spawn: args={args:?} cwd={}: {error}", root.display()),
            )
        })?;
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
    std::process::ExitStatus::from_raw(u32::from(!success))
}
