use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

use super::candidate::{make_candidate_proven_windows_package, run_source_projection};
use super::{complete_archive_fixture, complete_plugin_fixture};

pub(super) type CandidateTestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "candidate_admission.rs"]
pub(super) mod candidate_admission;
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
        (
            "bundled_platforms=\\\ndarwin-arm64 linux-x86_64 windows-x86_64",
            false,
        ),
        ("cat <<'", false),
    ] {
        assert_eq!(
            projection(appended).status.success(),
            succeeds,
            "{appended}"
        );
    }
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

// Keep assembler and archive fixtures aligned on the canonical Watcher platform inventory.
pub(super) struct CandidateAssemblyFixture {
    temp: TempDir,
    source_commit: String,
}

impl CandidateAssemblyFixture {
    pub(super) fn new() -> CandidateTestResult<Self> {
        let temp = tempdir()?;
        let root = temp.path();
        crate::support::release_archive::copy_tree(&fixture_seed::candidate_fixture_seed()?, root)?;
        // Keep real package delegates and metadata so the Rust artifact validator checks a complete staged plugin.
        let repository = codexy_runtime::paths::repository_root();
        crate::support::release_archive::copy_tree(
            &repository.join("plugins/codexy-devtools"),
            &root.join("plugins/codexy-devtools"),
        )?;
        // Keep authentic versions, MCP pairs, wrapper pins, and handoff inputs for activation.
        let core_plugin = repository.join("plugins/codexy");
        crate::support::release_archive::copy_tree(&core_plugin, &root.join("plugins/codexy"))?;
        for relative in [
            ".agents/plugins/release-publish-contract.json",
            "packages/codexy-runtime/src/version/bootstrap.rs",
        ] {
            let target = root.join(relative);
            fs::create_dir_all(target.parent().ok_or("activation fixture parent")?)?;
            fs::copy(repository.join(relative), target)?;
        }
        // The seed rsync stub must honor the assembler's excluded receipts, not copy source-selected metadata.
        fs::write(
            root.join("test-bin/rsync"),
            "#!/bin/sh\nset -eu\nsource=${8:?source}\ndestination=${9:?destination}\nmkdir -p \"$destination\"\ncp -R \"${source%/}/.\" \"$destination/\"\nrm -rf \"$destination/runtime\" \"$destination/runtime-release.json\" \"$destination/runtime-candidate.json\"\n",
        )?;
        seed_reader_runtime_inputs(root)?;
        // Preserve the selected-runtime launcher while narrowing its source-projection declaration.
        let wrapper = root.join("plugins/codexy-devtools/mcp/codexy-mcp-devtools");
        let text = fs::read_to_string(&wrapper)?;
        fs::write(
            &wrapper,
            text.replace(
                "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
                "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            ),
        )?;
        run_git(
            root,
            &[
                "add",
                ".agents/plugins/release-publish-contract.json",
                "plugins/codexy-devtools/mcp",
            ],
        )?;
        run_git(root, &["commit", "-qm", "candidate fixture"])?;
        let source_commit = String::from_utf8(run_git(root, &["rev-parse", "HEAD"])?)?;
        Ok(Self {
            temp,
            source_commit: source_commit.trim().into(),
        })
    }

    pub(super) fn enable_core_runtime(&self) -> CandidateTestResult {
        let repository = codexy_runtime::paths::repository_root();
        for (platform, extension, _) in super::candidate::watchers::WATCHERS {
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
        let target_version = candidate_admission::candidate_version(self.root())
            .expect("admitted candidate version");
        Command::new("sh")
            .arg("scripts/assemble-runtime-candidate")
            .current_dir(self.root())
            .env("SOURCE_COMMIT", &self.source_commit)
            .env("STAGING_RUN_ID", "1")
            .env("STAGING_RUN_ATTEMPT", "1")
            .env("GITHUB_SERVER_URL", "https://github.com")
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("TARGET_VERSION", target_version)
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

pub(super) fn core_assembly_fixture() -> CandidateTestResult<CandidateAssemblyFixture> {
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

fn run_git(root: &Path, arguments: &[&str]) -> CandidateTestResult<Vec<u8>> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()?;
    assert!(
        output.status.success(),
        "git: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn seed_reader_runtime_inputs(root: &Path) -> CandidateTestResult {
    let binaries = complete_plugin_fixture(&root.join("reader-fixtures"))?;
    super::candidate::make_core_candidate(&binaries, true, false);
    let staged = root.join("staged-runtime");
    // Reuse valid runtime signatures so readers validate the archive without executing these fixtures.
    crate::support::release_archive::copy_tree(&binaries.join("runtime"), &staged)?;
    fs::copy(
        binaries.join("mcp/codexy-mcp-devtools.exe"),
        staged.join("codexy-mcp-devtools-windows-x86_64.exe"),
    )?;
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
