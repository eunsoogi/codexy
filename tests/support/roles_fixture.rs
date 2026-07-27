#[cfg(windows)]
use std::path::{Path, PathBuf};

use super::PluginFixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[cfg(windows)]
const MUTABLE_SENTINEL: &str = "agents/codexy-sentinel.toml";

pub(crate) fn roles_fixture() -> TestResult<PluginFixture> {
    #[cfg(not(windows))]
    return super::plugin_fixture();

    #[cfg(windows)]
    {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("codexy");
        copy_roles_fixture(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &root,
            Path::new(""),
        )?;
        return Ok(PluginFixture::from_parts(temp, root));
    }
}

#[cfg(windows)]
fn copy_roles_fixture(source: &Path, target: &Path, relative: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let entry_relative: PathBuf = relative.join(entry.file_name());
        if source_path.is_dir() {
            copy_roles_fixture(&source_path, &target_path, &entry_relative)?;
        } else if entry_relative == Path::new(MUTABLE_SENTINEL) {
            std::fs::copy(&source_path, &target_path)?;
        } else if std::fs::hard_link(&source_path, &target_path).is_err() {
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}
