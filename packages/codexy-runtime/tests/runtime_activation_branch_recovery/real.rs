mod command;
mod metadata;
mod real_fixture;
mod real_fixture_seed;
mod real_source_pointer;
mod receipt;
mod shell_runner;

use real_fixture::Fixture;

#[test]
fn real_pre_671_committed_tree_authenticates_retry_and_metadata_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    let candidate = metadata::current_candidate_version()?;
    metadata::assert_canonical_default_prompt(&fixture.repo)?;
    metadata::assert_canonical_preserved_eol(&fixture.repo)?;
    real_source_pointer::assert_result(fixture.verify("main", &candidate)?, true, "exact retry");
    assert_eq!(
        fixture.cargo_invocations()?,
        0,
        "the verifier must use the injected prebuilt sync binary instead of cargo",
    );
    assert_eq!(
        fixture.external_activation_process_invocations(),
        1,
        "real matrix must retain only the successful verifier activation process",
    );
    Ok(())
}

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
