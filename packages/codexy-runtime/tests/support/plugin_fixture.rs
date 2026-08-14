use std::panic::Location;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct PluginFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    mutable_files: Vec<PathBuf>,
}

impl PluginFixture {
    pub(super) fn from_parts(
        temp: tempfile::TempDir,
        root: PathBuf,
        mutable_files: &[&Path],
    ) -> Self {
        let mut mutable_files = mutable_files
            .iter()
            .map(|path| super::plugin_fixture_mutable::normalized(path))
            .collect::<Vec<_>>();
        mutable_files.sort();
        mutable_files.dedup();
        Self {
            _temp: temp,
            root,
            mutable_files,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn reset_file(&self, relative: &Path) -> std::io::Result<()> {
        validate_relative_file(relative)?;
        if !self
            .mutable_files
            .contains(&super::plugin_fixture_mutable::normalized(relative))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "fixture reset path must be declared mutable",
            ));
        }
        let source = source_root().join(relative);
        let target = self.root.join(relative);
        clear_readonly(&target)?;
        std::fs::copy(source, &target)?;
        clear_readonly(&target)
    }
}

fn clear_readonly(path: &Path) -> std::io::Result<()> {
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(path, permissions)
}

#[track_caller]
pub(crate) fn plugin_fixture() -> TestResult<PluginFixture> {
    #[cfg(windows)]
    {
        return materialize_fixture(&[], fixture_identity("full", Location::caller()))
            .map_err(Into::into);
    }
    #[cfg(not(windows))]
    {
        super::profile_metrics::record("plugin_fixture");
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("codexy");
        super::copy_dir(source_root(), &root)?;
        materialize_admission_runtime_suite(&root)?;
        Ok(PluginFixture::from_parts(temp, root, &[]))
    }
}

#[track_caller]
pub(crate) fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    #[cfg(windows)]
    {
        let fixture = materialize_fixture(&[], fixture_identity("full", Location::caller()))?;
        return Ok((fixture._temp, fixture.root));
    }
    #[cfg(not(windows))]
    {
        let fixture = plugin_fixture()?;
        Ok((fixture._temp, fixture.root))
    }
}

#[track_caller]
pub(crate) fn plugin_fixture_with_mutable_files(
    mutable_files: &[&Path],
) -> std::io::Result<PluginFixture> {
    for path in mutable_files {
        validate_relative_file(path)?;
    }
    materialize_fixture(mutable_files, fixture_identity("full", Location::caller()))
}

#[track_caller]
pub(crate) fn copy_plugin_fixture_with_mutable_files(
    mutable_files: &[&Path],
) -> std::io::Result<(tempfile::TempDir, PathBuf)> {
    #[cfg(windows)]
    let fixture = materialize_fixture(mutable_files, fixture_identity("full", Location::caller()))?;
    #[cfg(not(windows))]
    let fixture = plugin_fixture_with_mutable_files(mutable_files)?;
    Ok((fixture._temp, fixture.root))
}

#[track_caller]
pub(crate) fn copy_plugin_fixture_into_with_mutable_files(
    target: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    for path in mutable_files {
        validate_relative_file(path)?;
    }
    super::profile_metrics::record("plugin_fixture");
    super::plugin_fixture_copy::materialize(
        source_root(),
        target,
        mutable_files,
        &fixture_identity("full", Location::caller()),
    )?;
    materialize_admission_runtime_suite(target)?;
    super::plugin_fixture_mutable::record(target, mutable_files);
    Ok(())
}

#[track_caller]
pub(crate) fn roles_fixture() -> TestResult<PluginFixture> {
    #[cfg(windows)]
    {
        return materialize_fixture(
            &[
                Path::new("agents/codexy-inspector.toml"),
                Path::new("agents/codexy-sentinel.toml"),
            ],
            fixture_identity("full", Location::caller()),
        )
        .map_err(Into::into);
    }
    #[cfg(not(windows))]
    {
        core_fixture_with_mutable_files(&[Path::new("agents/codexy-sentinel.toml")])
            .map_err(Into::into)
    }
}

fn materialize_fixture(
    mutable_files: &[&Path],
    identity: String,
) -> std::io::Result<PluginFixture> {
    super::profile_metrics::record("plugin_fixture");
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    super::plugin_fixture_copy::materialize(source_root(), &root, mutable_files, &identity)?;
    materialize_admission_runtime_suite(&root)?;
    super::plugin_fixture_mutable::record(&root, mutable_files);
    Ok(PluginFixture::from_parts(temp, root, mutable_files))
}

fn fixture_identity(profile: &str, caller: &'static Location<'static>) -> String {
    let file = Path::new(caller.file());
    let relative = file
        .strip_prefix(codexy_runtime::paths::repository_root())
        .ok()
        .filter(|path| path.is_relative())
        .or_else(|| (!file.is_absolute()).then_some(file))
        .unwrap_or_else(|| Path::new(file.file_name().unwrap_or_default()));
    let path = relative.to_string_lossy().replace('\\', "/");
    format!("{profile}:{path}:{}", caller.line())
}

pub(crate) fn fixture_mutable_files(root: &Path) -> Option<Vec<PathBuf>> {
    super::plugin_fixture_mutable::files(root)
}

fn source_root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}

fn core_fixture_with_mutable_files(mutable_files: &[&Path]) -> std::io::Result<PluginFixture> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    super::plugin_fixture_copy::materialize(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &root,
        mutable_files,
        "full:core-roles",
    )?;
    super::plugin_fixture_mutable::record(&root, mutable_files);
    Ok(PluginFixture::from_parts(temp, root, mutable_files))
}

pub(crate) fn materialize_admission_runtime_suite(plugin_root: &Path) -> std::io::Result<()> {
    let repository = fixture_repository(plugin_root)?;
    let suite = repository.join("packages/codexy-runtime/tests/suites/all.rs");
    std::fs::create_dir_all(suite.parent().expect("suite parent"))?;
    std::fs::write(suite, "// admission runtime suite fixture\n")
}

fn fixture_repository(plugin_root: &Path) -> std::io::Result<&Path> {
    let parent = plugin_root.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture plugin root needs a parent",
        )
    })?;
    if parent.file_name().is_some_and(|name| name == "plugins") {
        parent.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "fixture plugin directory needs a repository parent",
            )
        })
    } else {
        Ok(parent)
    }
}

fn validate_relative_file(relative: &Path) -> std::io::Result<()> {
    validate_relative_file_at(relative, &source_root())
}

fn validate_relative_file_at(relative: &Path, source_root: &Path) -> std::io::Result<()> {
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
    let source = source_root.join(relative);
    if !source.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture mutable path must name a source regular file",
        ));
    }
    Ok(())
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
