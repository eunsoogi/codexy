use std::{fs, process::Output};

use crate::support;

#[path = "release_tag_admission/fixture.rs"]
mod fixture;

use fixture::{Fixture, RemoteTag};

#[test]
fn fixture_error_context_names_missing_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("missing fixture.sh");
    fixture::assert_fixture_error_context(&path, temp.path(), false)
}

#[cfg(unix)]
#[test]
fn fixture_error_context_names_non_executable_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("non-executable fixture.sh");
    fs::write(&path, "#!/bin/sh\nexit 0\n")?;
    fixture::assert_fixture_error_context(&path, temp.path(), true)
}

#[test]
fn remote_version_tag_reconciliation_stays_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
    for state in [
        RemoteTag::Wrong,
        RemoteTag::Unpeelable,
        RemoteTag::Changed,
        RemoteTag::ExactAfterMainAdvance,
        RemoteTag::ExactOutsideProtectedMain,
        RemoteTag::ExactLosesProtectedMainAfterSource,
        RemoteTag::AbsentAfterMainAdvance,
        RemoteTag::Absent,
        RemoteTag::ConcurrentExact,
        RemoteTag::ConcurrentWrong,
        RemoteTag::ConcurrentUnpeelable,
        RemoteTag::ApiAuth,
        RemoteTag::ApiFailure,
    ] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "unsafe {state:?} tag unexpectedly admitted");
        assert_eq!(fixture.git_push_calls()?, 0, "{state:?} used unauthenticated git push");
        if matches!(
            state,
            RemoteTag::Wrong
                | RemoteTag::Unpeelable
                | RemoteTag::Changed
                | RemoteTag::ExactAfterMainAdvance
                | RemoteTag::ExactOutsideProtectedMain
                | RemoteTag::ExactLosesProtectedMainAfterSource
        ) {
            assert_eq!(fixture.release_calls()?, 0, "{state:?} reached release creation");
        }
    }
    let fixture = Fixture::new(RemoteTag::Exact)?;
    let output = fixture.run()?;
    assert!(!output.status.success(), "exact tag fixture crossed the release boundary");
    assert!(fixture.release_calls()? > 0, "exact tag did not reach draft release creation");
    assert_eq!(fixture.git_push_calls()?, 0, "exact tag used unauthenticated git push");
    Ok(())
}

#[test]
fn release_script_uses_the_immutable_draft_contract() -> Result<(), Box<dyn std::error::Error>> {
    let publisher = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"),
    )?;
    support::assert_structured_literals(
        &publisher,
        "immutable draft release creation",
        &[
            "draft_release_response=",
            "gh api --method POST --include",
            "repos/$GITHUB_REPOSITORY/releases",
            "-f \"tag_name=$RELEASE_TAG\"",
            "-f \"target_commitish=$ACTIVATION_COMMIT\"",
            "-f \"name=$RELEASE_TAG\"",
            "-f \"body=$changelog_notes\"",
            "-F draft=true",
            "-F prerelease=false",
            "release_create_diagnostic",
            "release_id_for_tag",
            "remote_tag_oid",
            "$tag_ref^{commit}",
            "scripts/reconcile-release-baseline",
        ],
    );
    let create = publisher.find("draft_release_response=").ok_or("draft release request")?;
    let upload = publisher.find("upload_release_asset").ok_or("asset upload")?;
    assert!(create < upload, "draft release must precede asset attachment");
    assert!(publisher.lines().all(|line| {
        let line = line.trim_start();
        line.starts_with('#') || !line.contains("git/refs")
    }));
    support::assert_structured_absent_literals(
        &publisher,
        "publisher must never publish while attaching assets",
        &["-F draft=false", "git push"],
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn draft_release_flow_checks_payload_tag_and_assets_before_baseline() -> Result<(), Box<dyn std::error::Error>> {
    let publisher = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"),
    )?;
    for (mode, success, status) in [("success", true, "201"), ("concurrent", true, "422"), ("401", false, "401"), ("422", false, "422"), ("500", false, "500")] {
        let (output, events) = run_release_fixture(&publisher, mode)?;
        assert_eq!(output.status.success(), success, "mode={mode}: stderr={} events={}", String::from_utf8_lossy(&output.stderr), events);
        assert!(!events.contains("git-ref-create"), "mode={mode} used standalone reference creation");
        if success {
            assert!(events.contains("payload-ok"), "mode={mode} did not validate exact draft payload");
            assert!(events.contains("tag-read-2"), "mode={mode} did not read back the created tag");
            assert!(events.contains("upload:codexy-marketplace-plugin.tar.gz"));
            assert!(events.contains("baseline"));
            assert!(events.find("release-create").unwrap() < events.find("upload:").unwrap());
            assert!(events.find("upload:").unwrap() < events.find("baseline").unwrap());
            assert!(!events.contains("publish"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains(&format!("create-release status={status}")), "mode={mode}: {stderr}");
            assert!(!stderr.contains("fixture-token"));
            assert!(stderr.len() < 700, "mode={mode} diagnostic exceeded bound");
        }
    }
    for mode in ["non-draft", "wrong-target", "duplicate"] {
        let (output, events) = run_release_fixture(&publisher, mode)?;
        assert!(!output.status.success(), "mode={mode} was admitted");
        assert!(!events.contains("upload:"), "mode={mode} reached asset attachment");
    }
    Ok(())
}

#[test]
fn fixture_discards_every_inherited_git_and_github_state() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
    let output = fixture.run_with_inherited_state(&[
        ("GIT_DIR", "host-git-dir"),
        ("GIT_WORK_TREE", "host-work-tree"),
        ("GIT_INDEX_FILE", "host-index"),
        ("GIT_COMMON_DIR", "host-common"),
        ("GH_CONFIG_DIR", "host-gh-config"),
        ("GH_HOST", "host-gh"),
        ("GH_ENTERPRISE_TOKEN", "host-enterprise-token"),
        ("GH_TOKEN", "host-gh-token"),
        ("GITHUB_TOKEN", "host-token"),
    ])?;
    assert!(!output.status.success());
    assert_eq!(fixture.git_push_calls()?, 0);
    assert!(fixture.command_calls("git")? >= 5);
    assert!(fixture.command_calls("gh")? >= 2);
    Ok(())
}

#[cfg(unix)]
fn run_release_fixture(publisher: &str, mode: &str) -> Result<(Output, String), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin)?;
    fs::create_dir_all(root.join("dist"))?;
    fs::create_dir_all(root.join("scripts"))?;
    for name in ["codexy-marketplace-plugin.tar.gz", "codexy-marketplace-bundle.tar.gz", "codexy-runtime-package.tar.gz"] {
        fs::write(root.join("dist").join(name), format!("bytes:{name}"))?;
    }
    fs::write(root.join("dist/runtime-release-receipt.json"), "{\"source\":{\"stagingSourceCommit\":\"0123456789abcdef0123456789abcdef01234567\",\"activationCommit\":\"89abcdef0123456789abcdef0123456789abcdef\"},\"staging\":{\"runId\":42}}")?;
    let script = root.join("publish.sh");
    fs::write(&script, publisher.replace("scripts/generate-release-changelog \"$RELEASE_TAG\"", "printf notes"))?;
    support::make_executable(&script)?;
    let baseline = root.join("scripts/reconcile-release-baseline");
    fs::write(&baseline, "#!/bin/sh\nprintf '%s\\n' baseline >> events\n")?;
    support::make_executable(&baseline)?;
    fs::write(root.join("tag-reads"), "0")?;
    fs::write(root.join("uploads"), "0")?;
    if matches!(mode, "non-draft" | "wrong-target" | "duplicate") {
        fs::write(root.join("tag-created"), "")?;
    }
    fs::write(bin.join("git"), lifecycle_git())?;
    fs::write(bin.join("gh"), lifecycle_gh())?;
    support::make_executable(&bin.join("git"))?;
    support::make_executable(&bin.join("gh"))?;
    let host_path = std::env::var_os("PATH").ok_or("PATH")?;
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&host_path)))?;
    let output = std::process::Command::new("sh")
        .arg(&script)
        .current_dir(root)
        .env("PATH", path)
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .env("GH_TOKEN", "fixture-token")
        .env("RELEASE_MODE", mode)
        .env("TAG_CREATED", root.join("tag-created"))
        .env("TAG_READS", root.join("tag-reads"))
        .env("UPLOADS", root.join("uploads"))
        .env("EVENTS", root.join("events"))
        .env("STAGING_SOURCE_COMMIT", "0123456789abcdef0123456789abcdef01234567")
        .env("ACTIVATION_COMMIT", "89abcdef0123456789abcdef0123456789abcdef")
        .env("STAGING_RUN_ID", "42")
        .env("RELEASE_TAG", "v1.3.0")
        .output()?;
    Ok((output, fs::read_to_string(root.join("events")).unwrap_or_default()))
}

#[cfg(unix)]
fn lifecycle_git() -> &'static str {
    "#!/bin/sh\nset -eu\ncase \"$1\" in fetch|merge-base) exit 0 ;; rev-parse) case \"$*\" in *origin/main*|*refs/tags/*) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; *) printf '%s\\n' \"$2\" ;; esac ;; ls-remote) n=$(cat \"$TAG_READS\"); n=$((n + 1)); printf '%s\\n' \"tag-read-$n\" >> \"$EVENTS\"; printf '%s\\n' \"$n\" > \"$TAG_READS\"; if test -f \"$TAG_CREATED\"; then printf '%s\\trefs/tags/%s\\n' \"$ACTIVATION_COMMIT\" \"$RELEASE_TAG\"; fi; exit 0 ;; push) printf '%s\\n' git-push >> \"$EVENTS\"; exit 1 ;; *) exit 1 ;; esac\n"
}

#[cfg(unix)]
fn lifecycle_gh() -> &'static str {
    r#"#!/bin/sh
set -eu
state() { n=$(cat "$UPLOADS"); assets='[]'; test "$n" -ge 4 && assets='[{"id":1,"name":"codexy-marketplace-plugin.tar.gz","size":1,"digest":"sha256:plugin"},{"id":2,"name":"codexy-marketplace-bundle.tar.gz","size":1,"digest":"sha256:bundle"},{"id":3,"name":"codexy-runtime-package.tar.gz","size":1,"digest":"sha256:runtime"},{"id":4,"name":"runtime-release-receipt.json","size":1,"digest":"sha256:receipt"}]'; target="$ACTIVATION_COMMIT"; draft=true; test "$RELEASE_MODE" = wrong-target && target=0123456789abcdef0123456789abcdef01234567; test "$RELEASE_MODE" = non-draft && draft=false; printf '{"id":42,"name":"%s","tag_name":"%s","target_commitish":"%s","draft":%s,"prerelease":false,"upload_url":"https://uploads.example/repos/eunsoogi/codexy/releases/42/assets{?name,label}","assets":%s}\n' "$RELEASE_TAG" "$RELEASE_TAG" "$target" "$draft" "$assets"; }
printf 'gh:%s\n' "$*" >> "$EVENTS"
if [ "$1" = release ]; then case "$RELEASE_MODE" in non-draft|wrong-target|duplicate) state ;; *) printf '%s\n' release-view >> "$EVENTS"; exit 1 ;; esac; fi
if [ "$1" != api ]; then exit 1; fi
case "$*" in
  *"releases/42/assets?name="*) name=$(printf '%s\n' "$*" | sed -E 's/.*[?]name=([^ ]*).*/\1/'); printf '%s\n' "upload:$name" >> "$EVENTS"; n=$(cat "$UPLOADS"); printf '%s\n' $((n + 1)) > "$UPLOADS"; printf '{}\n' ;;
  *"repos/eunsoogi/codexy/releases/42"*) state ;;
  *"repos/eunsoogi/codexy/releases?per_page=100"*) case "$RELEASE_MODE" in concurrent|non-draft|wrong-target) printf '[%s]\n' "$(state)" ;; duplicate) printf '[%s,%s]\n' "$(state)" "$(state)" ;; *) printf '[]\n' ;; esac ;;
  *"--method POST"*"repos/eunsoogi/codexy/releases "*)
    case "$*" in *"-f tag_name=v1.3.0"*"-f target_commitish=89abcdef0123456789abcdef0123456789abcdef"*"-f name=v1.3.0"*"-f body=notes"*) printf '%s\n' payload-ok >> "$EVENTS" ;; *) exit 88 ;; esac
    printf '%s\n' release-create >> "$EVENTS"; : > "$TAG_CREATED"
    case "$RELEASE_MODE" in success) printf 'HTTP/2.0 201 Created\nContent-Type: application/json\n\n%s' "$(state)" ;; concurrent) printf 'HTTP/2.0 422 Unprocessable Entity\n\n{"message":"fixture-token Reference update failed"}\n'; exit 1 ;; 401|422|500) printf 'HTTP/2.0 %s Failure\n\n{"message":"fixture-token failure"}\n' "$RELEASE_MODE"; exit 1 ;; esac ;;
  *"releases/assets/1"*) printf 'bytes:codexy-marketplace-plugin.tar.gz' ;;
  *"releases/assets/2"*) printf 'bytes:codexy-marketplace-bundle.tar.gz' ;;
  *"releases/assets/3"*) printf 'bytes:codexy-runtime-package.tar.gz' ;;
  *"releases/assets/4"*) printf '{"source":{"stagingSourceCommit":"0123456789abcdef0123456789abcdef01234567","activationCommit":"89abcdef0123456789abcdef0123456789abcdef"},"staging":{"runId":42}}' ;;
  *) exit 1 ;;
esac
"#
}
