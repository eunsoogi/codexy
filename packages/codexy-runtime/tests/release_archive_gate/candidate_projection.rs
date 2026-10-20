use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::{tempdir, TempDir};

use super::candidate::{make_candidate_proven_windows_package, run_source_projection};
use super::{complete_archive_fixture, complete_plugin_fixture};

#[path = "../runtime_candidate_assembly_contract/fixture_seed.rs"]
mod fixture_seed;

fn projection(appended: &str) -> std::process::Output {
    let root = tempdir().expect("candidate projection root");
    let plugin = complete_plugin_fixture(root.path()).expect("candidate plugin fixture");
    make_candidate_proven_windows_package(&plugin);
    let wrapper = plugin.join("mcp/codexy-mcp-devtools");
    let text = std::fs::read_to_string(&wrapper).expect("candidate wrapper");
    std::fs::write(&wrapper, format!("{text}\n{appended}\n")).expect("wrapper mutation");
    run_source_projection(&plugin)
}

#[test]
// An active assignment changes the candidate contract, while identical text printed as data must remain inert.
fn source_projection_rejects_executable_platform_mutations_and_ignores_inert_text() {
    for (appended, succeeds) in [
        (
            "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            false,
        ),
        (
            "export bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            false,
        ),
        ("eval 'bundled_platforms=darwin-arm64'", false),
        ("printf '%s\\n' 'bundled_platforms=darwin-arm64'", true),
    ] {
        assert_eq!(
            projection(appended).status.success(),
            succeeds,
            "{appended}"
        );
    }
}

#[test]
fn source_projection_rejects_multiline_and_malformed_heredoc_declarations() {
    assert!(
        !projection("bundled_platforms=\\\ndarwin-arm64 linux-x86_64 windows-x86_64")
            .status
            .success()
    );
    assert!(!projection("cat <<'").status.success());
}

#[test]
fn archive_gate_rejects_a_candidate_runtime_path_outside_its_contract() {
    let (root, plugin_root, archive) = complete_archive_fixture("candidate-runtime-path");
    super::candidate::make_candidate_proven_windows_package(&plugin_root);
    let release_path = plugin_root.join("runtime-release.json");
    let mut release: serde_json::Value =
        serde_json::from_slice(&fs::read(&release_path).expect("release contract"))
            .expect("release contract JSON");
    release["platforms"]["windows-x86_64"]["lsp"]["path"] =
        serde_json::json!("runtime/codexy-mcp-lsp-untrusted.exe");
    fs::write(
        &release_path,
        serde_json::to_vec_pretty(&release).expect("release JSON"),
    )
    .expect("malformed candidate release");
    super::create_archive(root.path(), &archive).expect("candidate archive");
    let status = super::candidate::run_candidate_gate(root.path(), &archive, &plugin_root).status;
    assert!(!status.success());
}

#[cfg(unix)]
#[test]
fn candidate_assembler_rejects_watcher_symlink_hash_input() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = core_assembly_fixture()?;
    let path = fixture
        .root()
        .join("staged-runtime/codexy-mcp-watcher-darwin-arm64.bin");
    fs::remove_file(&path)?;
    // Reject links before hashing so external or missing targets cannot enter the candidate.
    std::os::unix::fs::symlink("missing-watcher", &path)?;
    let output = fixture.assemble();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("core-watcher runtime must be a regular file"));
    Ok(())
}

// Keep assembler fixtures in this owned test module; the shared fixture uses a private suite export.
pub(super) struct CandidateAssemblyFixture {
    temp: TempDir,
    source_commit: String,
}

impl CandidateAssemblyFixture {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();
        copy_fixture_tree(&fixture_seed::candidate_fixture_seed()?, root)?;
        // Keep real package delegates and metadata so the Rust artifact validator checks a complete staged plugin.
        copy_fixture_tree(
            &codexy_runtime::paths::repository_root().join("plugins/codexy-devtools"),
            &root.join("plugins/codexy-devtools"),
        )?;
        super::candidate::watchers::seed_reader_runtime_inputs(root)?;
        fs::write(
            root.join("plugins/codexy-devtools/mcp/codexy-mcp-devtools"),
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"\n",
        )?;
        run_git(root, &["add", "plugins/codexy-devtools/mcp"])?;
        run_git(root, &["commit", "-qm", "candidate fixture"])?;
        let source_commit = String::from_utf8(run_git(root, &["rev-parse", "HEAD"])?)?;
        Ok(Self {
            temp,
            source_commit: source_commit.trim().into(),
        })
    }

    pub(super) fn enable_core_runtime(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repository = codexy_runtime::paths::repository_root();
        for (platform, extension) in [
            ("darwin-arm64", "bin"),
            ("linux-x86_64", "bin"),
            ("windows-x86_64", "exe"),
        ] {
            fs::write(
                self.root().join(format!(
                    "staged-runtime/codexy-handoff-validate-{platform}.{extension}"
                )),
                format!("handoff-{platform}\n"),
            )?;
        }
        for relative in [
            "packages/codexy-runtime/schemas/handoff-runtime.schema.json",
            "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh",
            "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd",
            "plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py",
            "scripts/handoff_runtime_contract.py",
        ] {
            let target = self.root().join(relative);
            fs::create_dir_all(target.parent().ok_or("core source parent")?)?;
            fs::copy(repository.join(relative), target)?;
        }
        Ok(())
    }

    pub(super) fn root(&self) -> &Path {
        self.temp.path()
    }

    pub(super) fn assemble(&self) -> Output {
        Command::new("sh")
            .arg("scripts/assemble-runtime-candidate")
            .current_dir(self.root())
            .env("SOURCE_COMMIT", &self.source_commit)
            .env("STAGING_RUN_ID", "1")
            .env("STAGING_RUN_ATTEMPT", "1")
            .env("GITHUB_SERVER_URL", "https://github.invalid")
            .env("GITHUB_REPOSITORY", "example/codexy")
            .env("TARGET_VERSION", "1.6.0")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.root().join("test-bin").display(),
                    std::env::var("PATH").expect("PATH")
                ),
            )
            .output()
            .expect("candidate assembly starts")
    }
}

pub(super) fn core_assembly_fixture() -> Result<CandidateAssemblyFixture, Box<dyn std::error::Error>>
{
    let fixture = CandidateAssemblyFixture::new()?;
    fixture.enable_core_runtime()?;
    let test_bin = fixture.root().join("test-bin");
    let tar = Command::new("tar").arg("--version").output().ok();
    if !tar.as_ref().is_some_and(|output| {
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("GNU tar")
    }) {
        // The seed tar stub cannot emit an artifact; use Python to produce one off GNU hosts.
        fs::write(test_bin.join("tar"), PYTHON_TAR_FALLBACK)?;
    } else {
        fs::remove_file(test_bin.join("tar"))?;
    }
    if Command::new("sha256sum").arg("--version").output().is_err() {
        let checksum = test_bin.join("sha256sum");
        fs::write(&checksum, PYTHON_SHA256SUM_FALLBACK)?;
        super::make_executable(&checksum)?;
    }
    Ok(fixture)
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

pub(super) fn copy_fixture_tree(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_fixture_tree(&source_path, &target_path)?;
        } else {
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

const PYTHON_SHA256SUM_FALLBACK: &str = "#!/usr/bin/env python3\nimport hashlib, pathlib, sys\nfor name in sys.argv[1:]:\n print(hashlib.sha256(pathlib.Path(name).read_bytes()).hexdigest(), name)\n";

const PYTHON_TAR_FALLBACK: &str = r#"#!/usr/bin/env python3
import os
import sys
import tarfile

arguments = sys.argv[1:]
base = arguments[arguments.index("-C") + 1]
archive = arguments[arguments.index("-czf") + 1]
with tarfile.open(archive, "w:gz") as output:
    for name in arguments[arguments.index("-czf") + 2:]:
        output.add(os.path.join(base, name), arcname=name)
"#;
