use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct PluginFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl PluginFixture {
    pub(super) fn from_parts(temp: tempfile::TempDir, root: PathBuf) -> Self {
        Self { _temp: temp, root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn reset_file(&self, relative: &Path) -> std::io::Result<()> {
        validate_relative_file(relative)?;
        let source = source_root().join(relative);
        std::fs::copy(source, self.root.join(relative)).map(|_| ())
    }
}

pub(crate) fn plugin_fixture() -> TestResult<PluginFixture> {
    super::profile_metrics::record("plugin_fixture");
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    super::copy_dir(source_root(), &root)?;
    Ok(PluginFixture::from_parts(temp, root))
}

pub(crate) fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let fixture = plugin_fixture()?;
    Ok((fixture._temp, fixture.root))
}

pub(crate) fn plugin_fixture_with_mutable_files(
    mutable_files: &[&Path],
) -> std::io::Result<PluginFixture> {
    for path in mutable_files {
        validate_relative_file(path)?;
    }
    materialize_fixture(mutable_files)
}

pub(crate) fn copy_plugin_fixture_with_mutable_files(
    mutable_files: &[&Path],
) -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    let fixture = plugin_fixture_with_mutable_files(mutable_files)?;
    Ok((fixture._temp, fixture.root))
}

pub(crate) fn copy_plugin_fixture_into_with_mutable_files(
    target: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    for path in mutable_files {
        validate_relative_file(path)?;
    }
    super::profile_metrics::record("plugin_fixture");
    super::plugin_fixture_copy::materialize(source_root(), target, mutable_files)
}

pub(crate) fn roles_fixture() -> TestResult<PluginFixture> {
    plugin_fixture_with_mutable_files(&[Path::new("agents/codexy-sentinel.toml")])
        .map_err(Into::into)
}

fn materialize_fixture(mutable_files: &[&Path]) -> std::io::Result<PluginFixture> {
    super::profile_metrics::record("plugin_fixture");
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    super::plugin_fixture_copy::materialize(source_root(), &root, mutable_files)?;
    Ok(PluginFixture::from_parts(temp, root))
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}

fn validate_relative_file(relative: &Path) -> std::io::Result<()> {
    if !relative.is_relative()
        || relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture mutable path must be a relative regular file",
        ));
    }
    let source = source_root().join(relative);
    if !source.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture mutable path must name a source regular file",
        ));
    }
    Ok(())
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
