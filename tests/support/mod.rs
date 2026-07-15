#![allow(clippy::redundant_pub_crate)]
#![allow(dead_code, unused_imports)]

mod agent_model_assignments;
mod cache_fixture;
mod child_thread_ledger_skill;
mod package;
mod package_archive;
mod package_fixture;
mod release_cache;
mod release_cache_audit;
mod release_cache_fixture;
mod release_cache_git_fallback;
mod release_cache_release_match;
mod release_cache_resources;
mod release_version;
pub(crate) mod touched_loc;
pub(super) mod worktree_reservation_harness;
mod wrapper;

pub(crate) use agent_model_assignments::{
    TestResult, assert_privacy_diagnostic, public_contract_import_check,
    validate_agent_replacement, validate_catalog_replacement,
};
pub(crate) use child_thread_ledger_skill::{copy_plugin_fixture, stderr, validator};
pub(super) use package::{
    assert_wrapper_discovers_default_artifact_without_cargo,
    assert_wrapper_does_not_reuse_package_override_as_default_without_cargo,
    assert_wrapper_ignores_legacy_cache_before_default_package_refresh_without_cargo,
    assert_wrapper_installs_packaged_runtime_without_cargo,
    assert_wrapper_keeps_ref_override_exact_without_package_override,
    assert_wrapper_prefers_durable_default_package_without_cargo,
    assert_wrapper_refreshes_package_before_stale_cache_without_cargo,
    assert_wrapper_requires_token_for_default_artifact_without_cargo,
    assert_wrapper_reuses_cache_before_default_package_refresh_without_cargo,
};
pub(super) use release_cache::{
    assert_wrapper_ignores_unversioned_cache_before_default_package_refresh,
    assert_wrapper_refreshes_cached_runtime_when_plugin_release_changes,
};
pub(super) use release_cache_audit::{
    assert_wrapper_rejects_invalid_top_level_plugin_versions,
    assert_wrapper_reports_cache_helper_prerequisites,
    assert_wrapper_uses_top_level_version_in_minified_and_nested_manifests,
    assert_wrappers_migrate_v1_caches_without_deleting_them,
};
pub(super) use release_cache_git_fallback::assert_wrapper_reuses_default_package_git_fallback;
pub(super) use release_cache_release_match::{
    assert_wrapper_allows_explicit_package_release_mismatch,
    assert_wrapper_recovers_from_mismatched_cache_marker,
    assert_wrapper_recovers_from_poisoned_v2_cache_with_matching_release,
    assert_wrapper_rejects_stale_default_release_then_accepts_matching_release,
};
pub(super) use release_cache_resources::assert_wrapper_rejects_nonexecutable_helper_and_unavailable_manifest;
pub(crate) use wrapper::{
    WrapperCommandExt, WrapperFixture, assert_wrapper_uses_package_runtime_without_cargo, copy_dir,
    make_executable, run_wrapper, run_wrapper_command, run_wrapper_command_with_timeout,
    run_wrapper_with_optional_failure, wait_for_default_wrapper_output, wait_for_wrapper_output,
};
