use std::path::{Component, Path, PathBuf};
use std::time::Instant;

pub(crate) struct InstructionPolicyFixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    path: PathBuf,
    profile: FocusedFixtureProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FocusedFixtureProfile {
    files: u64,
    bytes: u64,
}

impl FocusedFixtureProfile {
    pub(crate) fn files(self) -> u64 {
        self.files
    }

    pub(crate) fn bytes(self) -> u64 {
        self.bytes
    }
}

impl InstructionPolicyFixture {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn profile(&self) -> FocusedFixtureProfile {
        self.profile
    }

    pub(crate) fn reset(&self) -> std::io::Result<()> {
        std::fs::copy(&self.source, &self.path).map(|_| ())
    }
}

pub(crate) fn instruction_policy_fixture(
    relative: &Path,
) -> std::io::Result<InstructionPolicyFixture> {
    validate_relative_surface(relative)?;
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("relative regular file parent"))?;
    let source = source_path(relative)?;
    let profile = FocusedFixtureProfile {
        files: 1,
        bytes: std::fs::metadata(&source)?.len(),
    };
    let started = Instant::now();
    std::fs::copy(&source, &path)?;
    super::profile_metrics::record_fixture_materialization(
        "selective:instruction-policy",
        profile.files,
        profile.bytes,
        started.elapsed().as_secs_f64(),
    );
    Ok(InstructionPolicyFixture {
        _temp: temp,
        source,
        path,
        profile,
    })
}

fn source_path(relative: &Path) -> std::io::Result<PathBuf> {
    let source = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(relative);
    source.is_file().then_some(source).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "instruction-policy fixture path must name a source regular file",
        )
    })
}

fn validate_relative_surface(relative: &Path) -> std::io::Result<()> {
    if !relative.is_relative()
        || relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "instruction-policy fixture path must be a relative regular file",
        ));
    }
    Ok(())
}
