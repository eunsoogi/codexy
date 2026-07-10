#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

use super::WrapperFixture;
use super::release_cache_audit::assert_wrapper_failure;

#[cfg(unix)]
pub(crate) fn assert_wrapper_rejects_nonexecutable_helper_and_unavailable_manifest(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let helper_fixture = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(helper_fixture.path())?;
    let helper = fixture.plugin_root.join("mcp/codexy-runtime-cache-key.py");
    let mut permissions = std::fs::metadata(&helper)?.permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&helper, permissions)?;
    assert_wrapper_failure(
        &fixture,
        server,
        &format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        "runtime cache helper is missing or not executable",
    )?;

    let manifest_fixture = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(manifest_fixture.path())?;
    // Removing the copied manifest proves the unavailable-manifest contract portably.
    std::fs::remove_file(fixture.plugin_root.join(".codex-plugin/plugin.json"))?;
    assert_wrapper_failure(
        &fixture,
        server,
        &format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        "cannot derive runtime cache key from plugin manifest",
    )?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn assert_wrapper_rejects_nonexecutable_helper_and_unavailable_manifest(
    _server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
