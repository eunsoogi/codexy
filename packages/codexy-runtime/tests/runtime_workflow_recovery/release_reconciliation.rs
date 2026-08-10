use std::{fs, path::Path};

use serde_yaml::Value;

#[path = "release_reconciliation_fixture.rs"]
mod fixture;

use fixture::Fixture;

const ASSETS: [&str; 3] = [
    "codexy-marketplace-plugin.tar.gz",
    "codexy-runtime-package.tar.gz",
    "runtime-release-receipt.json",
];
const GENERATED_NOTES: &str =
    "## Codexy v1.3.0\n\nChanges since v1.2.2:\n- restore generated changelog notes (12345678)";
const PARTIAL_NOTES: &str = "## Codexy v1.3.0\n\nChanges:";

#[test]
#[cfg(unix)]
fn release_reconciliation_recovers_only_exact_draft_assets()
-> Result<(), Box<dyn std::error::Error>> {
    let absent = Fixture::new("absent", &[])?;
    absent.assert_result(true, &["create", "upload", "upload", "upload", "notes", "publish"], false)?;

    let partial = Fixture::new("draft", &[ASSETS[0]])?;
    partial.assert_result(true, &["upload", "upload", "notes", "publish"], false)?;

    let stale_notes = Fixture::new("draft", &ASSETS)?;
    stale_notes.assert_generated_notes(&["notes", "publish"])?;
    let counterexample = Fixture::new("draft", &ASSETS)?;
    let runner = counterexample.reverse_finalization_order()?;
    let (_, log) = counterexample.run_path(&runner, false)?;
    assert_ne!(log, "notes\npublish\n", "publish-before-notes must fail the exact sequence");

    let published = Fixture::new("published", &ASSETS)?;
    published.assert_result(true, &[], false)?;

    let mismatch = Fixture::new("draft", &[ASSETS[0]])?;
    fs::write(mismatch.assets.join(ASSETS[0]), b"mismatch\n")?;
    mismatch.assert_result(false, &[], false)?;
    Ok(())
}

#[test]
fn counterexample_never_replaces_the_canonical_runner() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new("draft", &ASSETS)?;
    let runner = fixture.root.join("run-release-reconciliation");
    let canonical = fs::read_to_string(&runner)?;
    let counterexample = fixture.reverse_finalization_order()?;
    assert_ne!(counterexample, runner, "counterexample must use a distinct runner path");
    assert_eq!(fs::read_to_string(runner)?, canonical, "counterexample must not replace canonical runner");
    assert_ne!(fs::read_to_string(counterexample)?, canonical, "counterexample mutation must be applied");
    Ok(())
}

#[test]
#[cfg(unix)]
fn new_release_uses_generated_commit_log_notes() -> Result<(), Box<dyn std::error::Error>> {
    let absent = Fixture::new("absent", &[])?;
    absent.assert_generated_notes(&["create", "upload", "upload", "upload", "notes", "publish"])?;
    Ok(())
}

#[test]
#[cfg(unix)]
fn failed_changelog_generation_never_creates_a_release() -> Result<(), Box<dyn std::error::Error>> {
    let absent = Fixture::new("absent", &[])?;
    absent.assert_result(false, &[], true)?;
    Ok(())
}

#[test]
#[cfg(unix)]
fn failed_changelog_generation_never_publishes_stale_draft() -> Result<(), Box<dyn std::error::Error>> {
    let draft = Fixture::new("draft", &ASSETS)?;
    draft.assert_result(false, &[], true)?;
    Ok(())
}

fn reconciliation() -> Result<String, Box<dyn std::error::Error>> {
    let workflow =
        codexy_runtime::paths::repository_root().join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    let run = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Create and verify the only public version release")
        })
        .and_then(|step| step["run"].as_str())
        .ok_or("release reconciliation")?;
    Ok(run[run.find("if ! gh release view").ok_or("release view")?..].to_owned())
}

pub(super) fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn fake_gh() -> &'static str {
    r#"#!/bin/sh
set -eu
state=$(cat "$FAKE_RELEASE_STATE")
assets="$FAKE_RELEASE_ASSETS"
log="$FAKE_RELEASE_LOG"
case "$1 $2" in
  'release view')
    test "$state" != absent || exit 1
    draft=false; test "$state" = draft && draft=true
    printf '{"isDraft":%s,"assets":[' "$draft"
    separator=
    for asset in codexy-marketplace-plugin.tar.gz codexy-runtime-package.tar.gz runtime-release-receipt.json; do
      if test -f "$assets/$asset"; then printf '%s{"name":"%s"}' "$separator" "$asset"; separator=,; fi
    done
    printf ']}\n'
    ;;
  'release create') printf '%s\n' "$@" > "$FAKE_RELEASE_NOTES_ARGS"; printf '%s\n' draft > "$FAKE_RELEASE_STATE"; printf '%s\n' create >> "$log" ;;
  'release upload') asset=$(basename "$4"); cp "$4" "$assets/$asset"; printf '%s\n' upload >> "$log" ;;
  'release download')
    while test "$#" -gt 0; do
      case "$1" in --dir) directory=$2; shift 2 ;; --pattern) asset=$2; shift 2 ;; *) shift ;; esac
    done
    mkdir -p "$directory"; cp "$assets/$asset" "$directory/$asset"
    ;;
  'release edit') case "$*" in *--notes*) printf '%s\n' "$@" > "$FAKE_RELEASE_NOTES_ARGS"; printf '%s\n' notes >> "$log" ;; *--draft=false*) printf '%s\n' published > "$FAKE_RELEASE_STATE"; printf '%s\n' publish >> "$log" ;; *) exit 91 ;; esac ;;
  *) exit 91 ;;
esac
"#
}
