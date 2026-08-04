#[cfg(unix)]
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use serde_yaml::Value;
#[cfg(unix)]
use sha2::{Digest as _, Sha256};

#[cfg(unix)]
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
#[cfg(unix)]
fn public_release_selection_falls_back_only_when_release_is_confirmed_absent()
-> Result<(), Box<dyn std::error::Error>> {
    let public = Fixture::new("present", false)?;
    public.assert_result(true, false)?;

    let mismatch = Fixture::new("present", true)?;
    mismatch.assert_result(false, false)?;

    let staging = Fixture::new("absent", false)?;
    staging.assert_result(true, true)?;

    let unavailable = Fixture::new("error", false)?;
    unavailable.assert_result(false, false)?;
    Ok(())
}

#[cfg(unix)]
struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

#[cfg(unix)]
impl Fixture {
    fn new(release: &str, mismatch: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("durable selection fixture");
        let bin = root.join("bin");
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(&bin)?;
        fs::write(root.join("release-state"), release)?;
        let public = root.join("public.tar.gz");
        let staging = root.join("staging.tar.gz");
        fs::write(&public, b"public release bytes\n")?;
        fs::write(&staging, b"staged candidate bytes\n")?;
        let public_sha = digest(&public)?;
        let staging_sha = digest(&staging)?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            format!(r#"{{"candidate":{{"source":{{"commit":"{COMMIT}"}},"artifact":{{"stagingRunId":42,"stagingRunAttempt":3}}}},"artifact":{{"sha256":"{staging_sha}"}},"provenance":{{"runId":42}}}}"#),
        )?;
        fs::write(
            root.join(".agents/plugins/release-publish-contract.json"),
            r#"{"runtime":{"selectedTag":"v1.3.0"}}"#,
        )?;
        let provenance = if mismatch { r#"{"runId":99}"# } else { r#"{"runId":42}"# };
        fs::write(
            root.join("public-receipt.json"),
            format!(r#"{{"release":{{"tag":"v1.3.0"}},"source":{{"stagingSourceCommit":"{COMMIT}"}},"staging":{{"runId":42,"runAttempt":3}},"provenance":{provenance},"artifact":{{"sha256":"{public_sha}"}}}}"#),
        )?;
        let gh = bin.join("gh");
        fs::write(&gh, fake_gh())?;
        executable(&gh)?;
        let fallback = root.join("scripts/download-runtime-staging-artifact");
        fs::write(&fallback, "#!/bin/sh\nset -eu\nprintf '%s\\n' fallback >> \"$FALLBACK_LOG\"\nmkdir -p \"$1\"\ncp \"$STAGING_ARCHIVE\" \"$1/codexy-marketplace-plugin.tar.gz\"\n")?;
        executable(&fallback)?;
        let runner = root.join("run-selection");
        fs::write(&runner, format!("#!/bin/sh\nset -eu\n{}", selection()?) )?;
        executable(&runner)?;
        Ok(Self { _temporary: temporary, root })
    }

    fn assert_result(&self, success: bool, fallback: bool) -> Result<(), Box<dyn std::error::Error>> {
        let host_path = std::env::var_os("PATH").ok_or("PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(std::env::split_paths(&host_path));
        let output = Command::new(self.root.join("run-selection"))
            .current_dir(&self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("GITHUB_REPOSITORY_ID", "1")
            .env("PUBLIC_ARCHIVE", self.root.join("public.tar.gz"))
            .env("PUBLIC_RECEIPT", self.root.join("public-receipt.json"))
            .env("STAGING_ARCHIVE", self.root.join("staging.tar.gz"))
            .env("FALLBACK_LOG", self.root.join("fallback-log"))
            .env("FAKE_RELEASE_STATE", self.root.join("release-state"))
            .env("PATH", std::env::join_paths(paths)?)
            .output()?;
        assert_eq!(output.status.success(), success, "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(self.root.join("fallback-log").exists(), fallback);
        Ok(())
    }
}

#[cfg(unix)]
fn selection() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/plugin-runtime-binaries.yml");
    let parsed: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    parsed["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Download and verify selected immutable bytes"))
        .and_then(|step| step["run"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "selected immutable bytes step".into())
}

#[cfg(unix)]
fn digest(path: &Path) -> Result<String, std::io::Error> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

#[cfg(unix)]
fn executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(unix)]
fn fake_gh() -> &'static str {
    "#!/bin/sh\nset -eu\ncase \"$1 $2\" in\n  'release view')\n    case \"$(cat \"$FAKE_RELEASE_STATE\")\" in\n      present) ;;\n      absent) printf '%s\\n' 'release not found' >&2; exit 1 ;;\n      error) printf '%s\\n' 'HTTP 403 release lookup denied' >&2; exit 1 ;;\n    esac\n    ;;\n  'release download')\n    while test \"$#\" -gt 0; do case \"$1\" in --dir) directory=$2; shift 2 ;; --pattern) pattern=$2; mkdir -p \"$directory\"; case \"$pattern\" in codexy-marketplace-plugin.tar.gz) cp \"$PUBLIC_ARCHIVE\" \"$directory/$pattern\" ;; runtime-release-receipt.json) cp \"$PUBLIC_RECEIPT\" \"$directory/$pattern\" ;; esac; shift 2 ;; *) shift ;; esac; done\n    ;;\n  *) exit 91 ;;\nesac\n"
}
