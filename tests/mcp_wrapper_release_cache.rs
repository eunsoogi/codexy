mod support;

use support::{
    assert_wrapper_ignores_unversioned_cache_before_default_package_refresh,
    assert_wrapper_refreshes_cached_runtime_when_plugin_release_changes,
};

#[test]
fn wrappers_ignore_unversioned_cache_before_default_package_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_ignores_unversioned_cache_before_default_package_refresh(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_refresh_cached_runtimes_when_plugin_release_changes()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_refreshes_cached_runtime_when_plugin_release_changes(server)?;
    }
    Ok(())
}
