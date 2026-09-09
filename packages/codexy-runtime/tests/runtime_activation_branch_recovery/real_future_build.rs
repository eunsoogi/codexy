use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::command;

#[path = "real_future_dependency_cache.rs"]
mod dependency_cache;

pub(super) struct Binaries {
    pub(super) activator: PathBuf,
    pub(super) sync: PathBuf,
}

pub(super) struct BinaryBuilder {
    _temp: tempfile::TempDir,
    source: PathBuf,
    target: PathBuf,
}

impl BinaryBuilder {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("future-source");
        let mut clone = Command::new("git");
        clone
            .args(["clone", "--shared", "--no-tags", "--no-checkout"])
            .arg(codexy_runtime::paths::repository_root())
            .arg(&source);
        command::run(&mut clone)?;
        git(&source, &["checkout", "--detach", "HEAD"])?;
        let target = temp.path().join("cargo-target");
        dependency_cache::seed(&target)?;
        Ok(Self {
            target,
            _temp: temp,
            source,
        })
    }

    pub(super) fn build(
        &self,
        selected: &str,
        candidate: &str,
    ) -> Result<Binaries, Box<dyn std::error::Error>> {
        let path = self.source.join("packages/codexy-runtime/src/version/bootstrap.rs");
        let mut source = fs::read_to_string(&path)?;
        replace_constant(&mut source, "VERSION", selected)?;
        replace_constant(&mut source, "CANDIDATE_VERSION", candidate)?;
        fs::write(path, source)?;
        let manifest = self.source.join("packages/codexy-runtime/Cargo.toml");
        let output = Command::new("cargo")
            .args(["build", "--locked", "--quiet", "--profile", "test", "--manifest-path"])
            .arg(&manifest)
            .args(["--target-dir"])
            .arg(&self.target)
            .args(["--bin", "codexy-activate-runtime", "--bin", "codexy-sync-version"])
            .current_dir(&self.source)
            // Match outer dependency artifacts; incremental state stays private
            // and speeds up the second version of the actual runtime crate.
            .env("CARGO_PROFILE_TEST_INCREMENTAL", "true")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "future {candidate} binary build failed\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )
            .into());
        }
        let executable = |name: &str| {
            self.target
                .join("debug")
                .join(if cfg!(windows) {
                    format!("{name}.exe")
                } else {
                    name.to_owned()
                })
        };
        Ok(Binaries {
            activator: executable("codexy-activate-runtime"),
            sync: executable("codexy-sync-version"),
        })
    }
}

fn replace_constant(
    source: &mut String,
    name: &str,
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let prefix = format!("pub(super) const {name}: &str = \"");
    let starts = source.match_indices(&prefix).collect::<Vec<_>>();
    if starts.len() != 1 {
        return Err(format!("bootstrap source must contain exactly one {name} constant").into());
    }
    let value_start = starts[0].0 + prefix.len();
    let value_end = source[value_start..]
        .find("\";")
        .map(|offset| value_start + offset)
        .ok_or("bootstrap constant terminator")?;
    source.replace_range(value_start..value_end, version);
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    command::run(&mut command)
}
