mod command;
mod metadata;
mod real_fixture;
mod real_fixture_seed;
#[path = "real_source_pointer_support.rs"]
mod real_source_pointer_support;
mod receipt;
mod shell_runner;

mod real_source_pointer {
    pub(super) use super::real_source_pointer_support::{
        assert_activated_source_pointer, assert_result, restore_pre_activation_runtime_inputs,
    };
}

use real_fixture::Fixture;

#[test]
fn real_base_activator_preserves_candidate_bytes_with_autocrlf()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let candidate = metadata::current_candidate_version()?;
    metadata::enable_autocrlf(&fixture.repo)?;
    metadata::assert_canonical_preserved_eol(&fixture.repo)?;
    real_source_pointer::assert_result(fixture.verify("main", &candidate)?, true, "autocrlf retry");
    Ok(())
}
