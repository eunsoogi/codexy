#[cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use sha2::{Digest as _, Sha256};

#[cfg(unix)]
#[test]
fn legacy_selected_source_replays_missing_receipt_without_staging_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = LegacyFixture::new(LegacyContract::Valid)?;
    fixture.run(true)
}

#[cfg(unix)]
#[test]
fn legacy_selected_source_rejects_an_immutable_url_mismatch()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = LegacyFixture::new(LegacyContract::WrongUrl)?;
    fixture.run(false)
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum LegacyContract {
    Valid,
    WrongUrl,
}

#[cfg(unix)]
impl LegacyContract {
    fn url(self, selected_tag: &str) -> String {
        match self {
            Self::Valid => format!(
                "https://github.com/eunsoogi/codexy/releases/download/{selected_tag}/codexy-marketplace-plugin.tar.gz"
            ),
            Self::WrongUrl => String::from("https://example.invalid/runtime.tar.gz"),
        }
    }
}

#[cfg(unix)]
struct LegacyFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    selected_tag: String,
}

#[cfg(unix)]
impl LegacyFixture {
    fn new(contract: LegacyContract) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("legacy selected source fixture");
        let selected_tag = format!("fixture-legacy-selected-tag-{}", std::process::id());
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("plugins/codexy-devtools"))?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(root.join("bin"))?;
        let helper = codexy_runtime::paths::repository_root()
            .join("scripts/download-selected-runtime-package.sh");
        fs::copy(helper, root.join("scripts/download-selected-runtime-package.sh"))?;
        let public_archive = root.join("legacy-public.tar.gz");
        fs::write(&public_archive, b"immutable legacy public bytes\n")?;
        let staging_archive = root.join("staging.tar.gz");
        fs::write(&staging_archive, b"staging bytes that must not be selected\n")?;
        let digest = digest(&public_archive)?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            r#"{"candidate":{"source":{"commit":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"artifact":{"stagingRunId":42,"stagingRunAttempt":3}}}"#,
        )?;
        fs::write(
            root.join(".agents/plugins/release-publish-contract.json"),
            format!(r#"{{"runtime":{{"selectedTag":"{selected_tag}"}}}}"#),
        )?;
        fs::write(
            root.join("plugins/codexy-devtools/runtime-release.json"),
            format!(
                r#"{{"state":"legacy-public","artifact":{{"tag":"{selected_tag}","url":"{}","sha256":"{digest}"}}}}"#,
                contract.url(&selected_tag)
            ),
        )?;
        write_executable(&root.join("bin/gh"), fake_gh())?;
        write_executable(&root.join("bin/curl"), fake_curl())?;
        let fallback = root.join("scripts/download-runtime-staging-artifact");
        write_executable(
            &fallback,
            "#!/bin/sh\nset -eu\nprintf '%s\\n' fallback >> \"$FALLBACK_LOG\"\nmkdir -p \"$1\"\ncp \"$STAGING_ARCHIVE\" \"$1/codexy-marketplace-plugin.tar.gz\"\n",
        )?;
        let runner = root.join("run-selection");
        write_executable(
            &runner,
            "#!/bin/sh\nset -eu\nscripts/download-selected-runtime-package.sh dist/selected.tar.gz\n",
        )?;
        Ok(Self {
            _temporary: temporary,
            root,
            selected_tag,
        })
    }

    fn run(&self, expected_success: bool) -> Result<(), Box<dyn std::error::Error>> {
        let host_path = std::env::var_os("PATH").ok_or("PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(std::env::split_paths(&host_path));
        let output = Command::new(self.root.join("run-selection"))
            .current_dir(&self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("PUBLIC_ARCHIVE", self.root.join("legacy-public.tar.gz"))
            .env("LEGACY_ARCHIVE", self.root.join("legacy-public.tar.gz"))
            .env("STAGING_ARCHIVE", self.root.join("staging.tar.gz"))
            .env("FALLBACK_LOG", self.root.join("fallback-log"))
            .env("CURL_URL_LOG", self.root.join("curl-url-log"))
            .env("RELEASE_VIEW_LOG", self.root.join("release-view-log"))
            .env("PATH", std::env::join_paths(paths)? )
            .output()?;
        assert_eq!(
            output.status.success(),
            expected_success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(self.root.join("release-view-log"))?,
            "present\n"
        );
        assert!(self.root.join("public-release").is_dir());
        assert!(!self.root.join("fallback-log").exists());
        assert!(!self.root.join("public-release/runtime-release-receipt.json").exists());
        assert!(!self.root.join("dist/public-release").exists());
        if expected_success {
            let selected = self.root.join("dist/selected.tar.gz");
            assert_eq!(digest(&selected)?, digest(&self.root.join("legacy-public.tar.gz"))?);
            assert!(self.root.join("dist/legacy-public").is_file());
            assert_eq!(
                fs::read_to_string(self.root.join("curl-url-log"))?,
                format!(
                    "https://github.com/eunsoogi/codexy/releases/download/{}/codexy-marketplace-plugin.tar.gz\n",
                    self.selected_tag
                )
            );
        } else {
            assert!(!self.root.join("dist/selected.tar.gz").exists());
            assert!(!self.root.join("dist/legacy-public").exists());
            assert!(!self.root.join("curl-url-log").exists());
        }
        Ok(())
    }
}

#[cfg(unix)]
fn digest(path: &Path) -> Result<String, std::io::Error> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(unix)]
fn fake_gh() -> &'static str {
    "#!/bin/sh\nset -eu\ncase \"$1 $2\" in\n  'release view') printf '%s\\n' present > \"$RELEASE_VIEW_LOG\"; exit 0 ;;\n  'release download')\n    while test \"$#\" -gt 0; do\n      case \"$1\" in\n        --dir) directory=$2; shift 2 ;;\n        --pattern)\n          pattern=$2\n          if test \"$pattern\" = codexy-marketplace-plugin.tar.gz; then\n            mkdir -p \"$directory\"\n            cp \"$PUBLIC_ARCHIVE\" \"$directory/$pattern\"\n          fi\n          shift 2 ;;\n        *) shift ;;\n      esac\n    done\n    ;;\n  *) exit 91 ;;\nesac\n"
}

#[cfg(unix)]
fn fake_curl() -> &'static str {
    "#!/bin/sh\nset -eu\nwhile test \"$#\" -gt 0; do\n  case \"$1\" in\n    --fail|--location) shift ;;\n    -o) output=$2; shift 2 ;;\n    *) url=$1; shift ;;\n  esac\ndone\nprintf '%s\\n' \"$url\" > \"$CURL_URL_LOG\"\ncp \"$LEGACY_ARCHIVE\" \"$output\"\n"
}
