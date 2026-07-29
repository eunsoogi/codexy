use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static FIXTURE_MUTABLE_FILES: OnceLock<Mutex<BTreeMap<PathBuf, Vec<PathBuf>>>> = OnceLock::new();

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
    #[cfg(windows)]
    {
        return materialize_fixture(&[]).map_err(Into::into);
    }
    #[cfg(not(windows))]
    {
        super::profile_metrics::record("plugin_fixture");
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("codexy");
        super::copy_dir(source_root(), &root)?;
        materialize_admission_runtime_suite(&root)?;
        Ok(PluginFixture::from_parts(temp, root))
    }
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
    super::plugin_fixture_copy::materialize(source_root(), target, mutable_files)?;
    materialize_admission_runtime_suite(target)?;
    record_fixture_mutable_files(target, mutable_files);
    Ok(())
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
    materialize_admission_runtime_suite(&root)?;
    record_fixture_mutable_files(&root, mutable_files);
    Ok(PluginFixture::from_parts(temp, root))
}

pub(crate) fn fixture_mutable_files(root: &Path) -> Option<Vec<PathBuf>> {
    FIXTURE_MUTABLE_FILES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .ok()
        .and_then(|fixtures| fixtures.get(root).cloned())
}

fn record_fixture_mutable_files(root: &Path, mutable_files: &[&Path]) {
    let mut declared = mutable_files
        .iter()
        .map(|path| normalized_relative_file(path))
        .collect::<Vec<_>>();
    declared.sort();
    declared.dedup();
    if let Ok(mut fixtures) = FIXTURE_MUTABLE_FILES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
    {
        fixtures.insert(root.to_path_buf(), declared);
    }
}

fn normalized_relative_file(path: &Path) -> PathBuf {
    path.components()
        .map(|component| component.as_os_str())
        .collect()
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy")
}

pub(crate) fn materialize_admission_runtime_suite(plugin_root: &Path) -> std::io::Result<()> {
    let repository = plugin_root.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture plugin root needs a parent",
        )
    })?;
    let suite = repository.join("tests/suites/all.rs");
    std::fs::create_dir_all(suite.parent().expect("suite parent"))?;
    std::fs::write(suite, "// admission runtime suite fixture\n")
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

#[cfg(all(test, windows))]
#[test]
fn records_mutable_paths_with_native_component_separators() {
    assert_eq!(
        normalized_relative_file(Path::new("agents/codexy-sentinel.toml")),
        Path::new("agents").join("codexy-sentinel.toml")
    );
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
