#[path = "release_publication_recovery/fixture.rs"]
mod fixture;
use fixture::{ASSETS, Fixture};

#[test]
fn publisher_baseline_and_finalizer_recover_fresh_partial_exact_and_public_states()
-> Result<(), Box<dyn std::error::Error>> {
    for (name, existing, published) in [
        ("fresh", &[][..], false),
        ("partial", &ASSETS[..1], false),
        ("exact rerun", &ASSETS[..], false),
    ] {
        let fixture = Fixture::new(existing, published, false)?;
        fixture.run_all()?;
        let published_log = fixture.log()?;
        fixture.run_all()?;
        assert_eq!(fixture.log()?, published_log, "{name} public rerun mutated release state");
        assert_eq!(fixture.assets()?, [ASSETS.as_slice(), &["release-baseline.json"]].concat(), "{name}");
        assert!(fixture.log()?.contains("publish"), "{name} did not finalize");
    }
    Ok(())
}

#[test]
fn finalizer_rejects_policy_drift_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&ASSETS, false, false)?;
    let publish = fixture.run("publish-verified-release")?;
    assert!(publish.status.success());
    let baseline_created = fixture.last_baseline_created()?;
    let before = fixture.log()?;
    let finalize = fixture.run_with_settings("finalize-verified-release", baseline_created, false)?;
    assert!(!finalize.status.success());
    assert!(fixture.log()?.starts_with(&before));
    assert!(!fixture.log()?.contains("publish\n"), "policy drift published the release");
    Ok(())
}

#[test]
fn finalizer_rejects_an_immutable_false_post_publication_observation()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&ASSETS, false, false)?;
    let publish = fixture.run("publish-verified-release")?;
    assert!(publish.status.success());
    let baseline_created = fixture.last_baseline_created()?;
    let finalize = fixture.run_with_policy("finalize-verified-release", baseline_created, true, false)?;
    assert!(!finalize.status.success());
    assert!(fixture.log()?.contains("publish\n"), "fixture did not exercise the post-publication observation");
    Ok(())
}

#[test]
fn mismatched_existing_asset_fails_before_any_upload_or_baseline_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[ASSETS[1]], false, true)?;
    let result = fixture.run("publish-verified-release")?;
    assert!(!result.status.success());
    assert!(fixture.log()?.is_empty(), "mismatch mutated release state");
    Ok(())
}
