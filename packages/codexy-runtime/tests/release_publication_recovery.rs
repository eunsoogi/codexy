#[path = "release_publication_recovery/fixture.rs"]
mod fixture;
use fixture::{ASSETS, Fixture};
use std::fs;

use crate::support::{
    FixtureCommand, FixtureScriptBinding, bind_posix_fixture_script_launchers,
    fixture_script_interpreter_path, write_posix_fixture_command,
};

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

#[test]
fn declared_release_child_launch_is_independent_of_the_shell_working_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture_root = temp.path().join("release-fixture");
    let scripts = fixture_root.join("scripts");
    fs::create_dir_all(&scripts)?;
    let parent = scripts.join("publisher");
    let child = scripts.join("release-helper");
    write_posix_fixture_command(&parent, "#!/bin/sh\nscripts/release-helper \"$1\"\n")?;
    write_posix_fixture_command(&child, "#!/bin/sh\nprintf 'release:%s\\n' \"$1\"\n")?;
    bind_posix_fixture_script_launchers(
        &parent,
        "FIXTURE_POSIX_SHELL",
        "FIXTURE_SCRIPT_ROOT",
        &[FixtureScriptBinding {
            invocation: "scripts/release-helper \"$1\"",
            child: "scripts/release-helper",
        }],
    )?;

    let output = FixtureCommand::new(&parent)
        .current_dir(temp.path())
        .arg("v9.9.9")
        .env_path("FIXTURE_POSIX_SHELL", fixture_script_interpreter_path(&parent)?)
        .env_path("FIXTURE_SCRIPT_ROOT", &fixture_root)
        .output()?;
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8(output.stdout)?, "release:v9.9.9\n");
    Ok(())
}
