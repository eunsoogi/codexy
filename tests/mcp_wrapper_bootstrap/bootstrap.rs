use super::*;

#[test]
fn lsp_wrapper_bootstraps_runtime_when_installed_without_bundled_binary()
-> Result<(), Box<dyn std::error::Error>> {
    assert_wrapper_bootstraps_runtime("lsp")
}

#[test]
fn codegraph_wrapper_bootstraps_runtime_when_installed_without_bundled_binary()
-> Result<(), Box<dyn std::error::Error>> {
    assert_wrapper_bootstraps_runtime("codegraph")
}

#[test]
fn wrappers_reuse_cached_runtime_for_moving_main_ref() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_reuses_moving_ref_runtime(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_use_rev_and_cache_for_pinned_sha_ref() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_uses_rev_for_pinned_sha_ref(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_fallback_to_cached_runtime_when_moving_ref_refresh_fails()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_falls_back_to_cached_runtime_after_refresh_failure(server)?;
    }
    Ok(())
}
