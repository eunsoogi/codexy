use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use crate::support;

use super::{fake_gh, make_executable, reconciliation, ASSETS, GENERATED_NOTES, PARTIAL_NOTES};

pub(super) struct Fixture {
    _temporary: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) assets: PathBuf,
    runner: PathBuf,
}

impl Fixture {
    pub(super) fn new(state: &str, existing: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("release reconciliation fixture");
        let bin = root.join("bin");
        let assets = root.join("release-assets");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&assets)?;
        fs::create_dir_all(root.join("dist"))?;
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts)?;
        let changelog = scripts.join("generate-release-changelog");
        fs::write(
            &changelog,
            "#!/bin/sh\nprintf '%s\\n' \"$CODEXY_GENERATED_RELEASE_NOTES\"\ntest \"${CODEXY_GENERATED_RELEASE_NOTES_FAIL:-false}\" != true\n",
        )?;
        make_executable(&changelog)?;
        fs::write(root.join("release-state"), state)?;
        for asset in ASSETS {
            fs::write(root.join("dist").join(asset), format!("verified {asset}\n"))?;
            if existing.contains(&asset) {
                fs::copy(root.join("dist").join(asset), assets.join(asset))?;
            }
        }
        let gh = bin.join("gh");
        fs::write(&gh, fake_gh())?;
        make_executable(&gh)?;
        let runner = root.join("run-release-reconciliation");
        write_executable(&runner, &format!("#!/bin/sh\nset -eu\n{}", reconciliation()?))?;
        Ok(Self {
            _temporary: temporary,
            root,
            assets,
            runner,
        })
    }

    pub(super) fn assert_generated_notes(&self, operations: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        self.assert_result(true, operations, false)?;
        let args = fs::read_to_string(self.root.join("release-notes-args"))?;
        let generated_notes_argument = format!("--notes\n{GENERATED_NOTES}\n");
        support::assert_structured_literals(&args, "generated release notes", &[&generated_notes_argument]);
        support::assert_structured_absent_literals(&args, "generated release notes", &["Verified version release."]);
        let log = fs::read_to_string(self.root.join("release-log"))?;
        support::assert_structured_literals(&log, "draft note update order", &["notes\npublish"]);
        Ok(())
    }

    pub(super) fn run_path(&self, runner: &Path, generator_fails: bool) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let host_path = std::env::var_os("PATH").ok_or("PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(std::env::split_paths(&host_path));
        let output = Command::new(runner)
            .current_dir(&self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("FAKE_RELEASE_STATE", self.root.join("release-state"))
            .env("FAKE_RELEASE_ASSETS", &self.assets)
            .env("FAKE_RELEASE_LOG", self.root.join("release-log"))
            .env("FAKE_RELEASE_NOTES_ARGS", self.root.join("release-notes-args"))
            .env("CODEXY_GENERATED_RELEASE_NOTES", if generator_fails { PARTIAL_NOTES } else { GENERATED_NOTES })
            .env("CODEXY_GENERATED_RELEASE_NOTES_FAIL", generator_fails.to_string())
            .env("PATH", std::env::join_paths(paths)?)
            .output()?;
        Ok((output.status.success(), fs::read_to_string(self.root.join("release-log")).unwrap_or_default()))
    }

    pub(super) fn assert_result(&self, success: bool, expected_operations: &[&str], generator_fails: bool) -> Result<(), Box<dyn std::error::Error>> {
        let (actual_success, log) = self.run_path(&self.runner, generator_fails)?;
        assert_eq!(actual_success, success, "release reconciliation failed");
        assert_eq!(log.lines().collect::<Vec<_>>(), expected_operations);
        Ok(())
    }

    pub(super) fn reverse_finalization_order(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let source = fs::read_to_string(&self.runner)?;
        let notes = "gh release edit v1.3.0 --notes \"$changelog_notes\"";
        let publish = "gh release edit v1.3.0 --draft=false";
        let counterexample = source.replace(&format!("{notes}\n  {publish}"), &format!("{publish}\n  {notes}"));
        assert_ne!(counterexample, source, "finalization counterexample must be applied");
        let path = self.root.join("run-release-reconciliation-counterexample");
        assert_ne!(path, self.runner, "counterexample must not reuse canonical runner path");
        write_executable(&path, &counterexample)?;
        Ok(path)
    }
}

fn write_executable(path: &Path, source: &str) -> std::io::Result<()> {
    if path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "fixture executable path must be new",
        ));
    }
    let staged = path.with_extension("new");
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&staged)?;
    file.write_all(source.as_bytes())?;
    drop(file);
    make_executable(&staged)?;
    fs::rename(staged, path)
}
