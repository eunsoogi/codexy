use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_yaml::Value;

use crate::support;

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
    counterexample.reverse_finalization_order()?;
    let (_, log) = counterexample.run(false)?;
    assert_ne!(log, "notes\npublish\n", "publish-before-notes must fail the exact sequence");

    let published = Fixture::new("published", &ASSETS)?;
    published.assert_result(true, &[], false)?;

    let mismatch = Fixture::new("draft", &[ASSETS[0]])?;
    fs::write(mismatch.assets.join(ASSETS[0]), b"mismatch\n")?;
    mismatch.assert_result(false, &[], false)?;
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

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    assets: PathBuf,
}

impl Fixture {
    fn new(state: &str, existing: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("release reconciliation fixture");
        let bin = root.join("bin");
        let assets = root.join("release-assets");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&assets)?;
        fs::create_dir_all(root.join("dist"))?;
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts)?;
        let changelog = scripts.join("generate-release-changelog");
        fs::write(
            &changelog,
            "#!/bin/sh\nprintf '%s\\n' \"$CODEXY_GENERATED_RELEASE_NOTES\"\ntest \"${CODEXY_GENERATED_RELEASE_NOTES_FAIL:-false}\" != true\n",
        )?;
        make_executable(&changelog)?;
        fs::write(root.join("release-state"), state)?;
        for asset in ASSETS {
            fs::write(root.join("dist").join(asset), format!("verified {asset}\n"))?;
            if existing.contains(&asset) {
                fs::copy(root.join("dist").join(asset), assets.join(asset))?;
            }
        }
        let gh = bin.join("gh");
        fs::write(&gh, fake_gh())?;
        make_executable(&gh)?;
        let runner = root.join("run-release-reconciliation");
        fs::write(
            &runner,
            format!("#!/bin/sh\nset -eu\n{}", reconciliation()?),
        )?;
        make_executable(&runner)?;
        Ok(Self {
            _temporary: temporary,
            root,
            assets,
        })
    }

    fn assert_generated_notes(&self, operations: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        self.assert_result(true, operations, false)?;
        let args = fs::read_to_string(self.root.join("release-notes-args"))?;
        let generated_notes_argument = format!("--notes\n{GENERATED_NOTES}\n");
        support::assert_structured_literals(
            &args,
            "generated release notes",
            &[&generated_notes_argument],
        );
        support::assert_structured_absent_literals(
            &args,
            "generated release notes",
            &["Verified version release."],
        );
        let log = fs::read_to_string(self.root.join("release-log"))?;
        support::assert_structured_literals(&log, "draft note update order", &["notes\npublish"]);
        Ok(())
    }

    fn run(&self, generator_fails: bool) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let host_path = std::env::var_os("PATH").ok_or("PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(std::env::split_paths(&host_path));
        let output = Command::new(self.root.join("run-release-reconciliation"))
            .current_dir(&self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("FAKE_RELEASE_STATE", self.root.join("release-state"))
            .env("FAKE_RELEASE_ASSETS", &self.assets)
            .env("FAKE_RELEASE_LOG", self.root.join("release-log"))
            .env(
                "FAKE_RELEASE_NOTES_ARGS",
                self.root.join("release-notes-args"),
            )
            .env(
                "CODEXY_GENERATED_RELEASE_NOTES",
                if generator_fails {
                    PARTIAL_NOTES
                } else {
                    GENERATED_NOTES
                },
            )
            .env(
                "CODEXY_GENERATED_RELEASE_NOTES_FAIL",
                generator_fails.to_string(),
            )
            .env("PATH", std::env::join_paths(paths)?)
            .output()?;
        Ok((
            output.status.success(),
            fs::read_to_string(self.root.join("release-log")).unwrap_or_default(),
        ))
    }

    fn assert_result(
        &self,
        success: bool,
        expected_operations: &[&str],
        generator_fails: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (actual_success, log) = self.run(generator_fails)?;
        assert_eq!(actual_success, success, "release reconciliation failed");
        assert_eq!(log.lines().collect::<Vec<_>>(), expected_operations);
        Ok(())
    }

    fn reverse_finalization_order(&self) -> Result<(), Box<dyn std::error::Error>> {
        let runner = self.root.join("run-release-reconciliation");
        let source = fs::read_to_string(&runner)?;
        let notes = "gh release edit v1.3.0 --notes \"$changelog_notes\"";
        let publish = "gh release edit v1.3.0 --draft=false";
        let counterexample = source.replace(&format!("{notes}\n  {publish}"), &format!("{publish}\n  {notes}"));
        assert_ne!(counterexample, source, "finalization counterexample must be applied");
        fs::write(runner, counterexample)?;
        Ok(())
    }
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

fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
