use std::path::Path;
use std::process::Output;

use crate::support;

pub(super) fn copy_plugin_fixture(
    mutable_files: &[&Path],
) -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    Ok(support::copy_plugin_fixture_with_mutable_files(mutable_files)?)
}

pub(super) fn normalized_fixture_stderr(output: &Output, path: &Path) -> String {
    String::from_utf8_lossy(&output.stderr)
        .replace(&path.display().to_string(), "<fixture-surface>")
}
