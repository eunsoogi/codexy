mod support;

use support::{
    assert_wrapper_ignores_unversioned_cache_before_default_package_refresh,
    assert_wrapper_refreshes_cached_runtime_when_plugin_release_changes,
    assert_wrapper_rejects_invalid_top_level_plugin_versions,
    assert_wrapper_reports_cache_helper_prerequisites,
    assert_wrapper_uses_top_level_version_in_minified_and_nested_manifests,
    assert_wrappers_migrate_v1_caches_without_deleting_them,
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

#[test]
fn wrappers_migrate_v1_caches_without_deleting_them() -> Result<(), Box<dyn std::error::Error>> {
    assert_wrappers_migrate_v1_caches_without_deleting_them()
}

#[test]
fn wrappers_use_top_level_versions_in_minified_and_nested_manifests()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_uses_top_level_version_in_minified_and_nested_manifests(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_reject_missing_or_invalid_top_level_versions() -> Result<(), Box<dyn std::error::Error>>
{
    for server in ["lsp", "codegraph"] {
        assert_wrapper_rejects_invalid_top_level_plugin_versions(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_report_missing_cache_helper_prerequisites() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_reports_cache_helper_prerequisites(server)?;
    }
    Ok(())
}
