use std::path::{Component, Path, PathBuf};

pub(crate) struct InstructionPolicyFixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    path: PathBuf,
}

impl InstructionPolicyFixture {
    pub(crate) fn path(&self) -> &Path {
        &self.path
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
    std::fs::copy(&source, &path)?;
    Ok(InstructionPolicyFixture {
        _temp: temp,
        source,
        path,
    })
}

fn source_path(relative: &Path) -> std::io::Result<PathBuf> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
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
