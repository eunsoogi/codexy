use super::*;

#[test]
fn wrappers_install_packaged_runtime_when_fresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_installs_packaged_runtime_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_discover_default_artifact_when_fresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_discovers_default_artifact_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_require_token_for_default_artifact_when_fresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_requires_token_for_default_artifact_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_prefer_durable_default_package_when_fresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_prefers_durable_default_package_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_reuse_cache_before_default_package_refresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_reuses_cache_before_default_package_refresh_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_ignore_legacy_cache_before_default_package_refresh_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_ignores_legacy_cache_before_default_package_refresh_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_do_not_reuse_package_override_as_default_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_does_not_reuse_package_override_as_default_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_refresh_package_before_stale_cache_without_cargo()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_refreshes_package_before_stale_cache_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_do_not_mask_runtime_ref_override_with_default_package()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_keeps_ref_override_exact_without_package_override(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_fail_when_moving_ref_initial_refresh_fails_without_cache()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_fails_without_cache_after_refresh_failure(server)?;
    }
    Ok(())
}
