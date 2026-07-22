#![allow(clippy::redundant_pub_crate)]
#![allow(dead_code, unused_imports)]

mod agent_model_assignments;
mod child_thread_ledger_skill;
mod package_archive;
mod release_archive;
pub(crate) mod routing_validator;
pub(crate) mod touched_loc;
pub(super) mod worktree_reservation_harness;
mod wrapper;
mod wrapper_copy;
mod wrapper_process;

pub(crate) use agent_model_assignments::{
    TestResult, assert_privacy_diagnostic, public_contract_import_check,
    validate_agent_replacement, validate_catalog_replacement,
};
pub(crate) use child_thread_ledger_skill::{
    copy_plugin_fixture, plugin_fixture, stderr, validator, validator_child_lane_ownership_file,
    validator_completion_handoff_files, validator_in_process, validator_instruction_policy,
    validator_routing,
};
pub(crate) use release_archive::assert_structured_literals;
pub(crate) use wrapper::{
    WrapperFixture, make_executable, next_bootstrap_version, published_bootstrap_version,
    run_wrapper_command, run_wrapper_command_with_timeout, wait_for_wrapper_output,
};
pub(crate) use wrapper_copy::copy_dir;
