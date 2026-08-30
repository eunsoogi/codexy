use std::{fs, process::Command};

#[path = "release_publication_recovery/fixture.rs"]
mod fixture;
use fixture::{gh_fixture, git_fixture, make_executable};

const ASSETS: [&str; 4] = [
    "codexy-marketplace-plugin.tar.gz",
    "codexy-marketplace-bundle.tar.gz",
    "codexy-runtime-package.tar.gz",
    "runtime-release-receipt.json",
];
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn attestation_set_fails_closed_and_sorts_complete_sets()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let bin = root.join("bin");
    let artifacts = root.join("artifacts");
    fs::create_dir_all(&bin)?;
    fs::create_dir_all(&artifacts)?;
    for name in ASSETS.into_iter().chain(["release-baseline.json"]) {
        fs::write(artifacts.join(name), name)?;
    }
    let gh = bin.join("gh");
    fs::write(&gh, "#!/bin/sh\nif test \"${FAIL_ATTESTATION:-false}\" = true; then exit 1; fi\nprintf '%s\\n' '[{\"verificationResult\":{\"statement\":{\"subject\":[{\"name\":\"artifact\"}]}}}]'\n")?;
    make_executable(&gh)?;
    let run = |mode: &str, output: &std::path::Path, fail: bool| {
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        let mut command = Command::new(codexy_runtime::paths::repository_root().join("scripts/verify-release-attestation-set"));
        command.args([artifacts.as_path(), output, std::path::Path::new(mode)])
            .env("PATH", path)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("ACTIVATION_COMMIT", COMMIT)
            .env("STAGING_SOURCE_COMMIT", COMMIT)
            .env("FAIL_ATTESTATION", fail.to_string())
            .output()
    };
    let failed_output = root.join("failed.json");
    assert!(!run("release", &failed_output, true)?.status.success());
    assert_ne!(fs::read_to_string(&failed_output).unwrap_or_default(), "[]\n");
    for (mode, expected) in [("release", &[ASSETS[1], ASSETS[0], ASSETS[2], ASSETS[3]][..]), ("baseline", &["release-baseline.json"][..])] {
        let output = root.join(format!("{mode}.json"));
        let result = run(mode, &output, false)?;
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        let records: Vec<serde_json::Value> = serde_json::from_slice(&fs::read(output)?)?;
        let names = records.iter().map(|record| record["name"].as_str().unwrap()).collect::<Vec<_>>();
        assert_eq!(names, expected);
        assert!(records.iter().all(|record| record["count"] == 1 && record["fingerprint"].as_str().is_some_and(|value| value.len() == 64)));
    }
    Ok(())
}

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
        assert!(fixture.reads()?.contains("api-download"), "{name} did not download assets by numeric release identity");
        assert!(fixture.log()?.contains("api-upload"), "{name} did not upload assets by numeric release identity");
        let published_log = fixture.log()?;
        fixture.run_all()?;
        assert_eq!(fixture.log()?, published_log, "{name} public rerun mutated release state");
        assert_eq!(fixture.assets()?, [ASSETS.as_slice(), &["release-baseline.json"]].concat(), "{name}");
        assert!(fixture.log()?.contains("publish"), "{name} did not finalize");
    }
    Ok(())
}

#[test]
fn finalizer_rejects_an_immutable_false_post_publication_observation()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&ASSETS, false, false)?;
    let publish = fixture.run("publish-verified-release")?;
    assert!(publish.status.success());
    let baseline_created = fixture.last_baseline_created()?;
    let finalize = fixture.run_with_policy("finalize-verified-release", baseline_created, false)?;
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

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl Fixture {
    fn new(
        existing: &[&str],
        published: bool,
        mismatch: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("release recovery fixture");
        fs::create_dir_all(root.join("bin"))?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(root.join("dist"))?;
        fs::create_dir_all(root.join("remote"))?;
        for asset in ASSETS {
            let bytes = if asset == "runtime-release-receipt.json" {
                format!("{{\"source\":{{\"stagingSourceCommit\":\"{COMMIT}\",\"activationCommit\":\"{COMMIT}\"}},\"staging\":{{\"runId\":\"42\"}}}}\n")
            } else {
                format!("verified {asset}\n")
            };
            fs::write(root.join("dist").join(asset), bytes)?;
        }
        for asset in existing {
            let bytes = if mismatch {
                b"wrong\n".to_vec()
            } else {
                fs::read(root.join("dist").join(asset))?
            };
            fs::write(root.join("remote").join(asset), bytes)?;
        }
        if !existing.is_empty() {
            fs::write(root.join("exists"), "yes")?;
        }
        fs::write(root.join("draft"), if published { "false" } else { "true" })?;
        for name in [
            "publish-verified-release",
            "reconcile-release-baseline",
            "finalize-verified-release",
        ] {
            let source = codexy_runtime::paths::repository_root()
                .join("scripts")
                .join(name);
            fs::copy(source, root.join("scripts").join(name))?;
        }
        fs::write(
            root.join("scripts/generate-release-changelog"),
            "#!/bin/sh\nprintf '%s\\n' notes\n",
        )?;
        fs::write(
            root.join("scripts/verify-release-attestation-set"),
            "#!/bin/sh\nprintf '[]\\n' > \"$2\"\n",
        )?;
        fs::write(root.join("bin/git"), git_fixture())?;
        fs::write(root.join("bin/gh"), gh_fixture())?;
        for path in fs::read_dir(root.join("scripts"))?.chain(fs::read_dir(root.join("bin"))?) {
            make_executable(&path?.path())?;
        }
        Ok(Self { _temp: temp, root })
    }

    fn run_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let publish = self.run("publish-verified-release")?;
        assert!(
            publish.status.success(),
            "stdout: {} stderr: {}",
            String::from_utf8_lossy(&publish.stdout),
            String::from_utf8_lossy(&publish.stderr)
        );
        let baseline_created = self.last_baseline_created()?;
        let finalize = self.run_with_policy("finalize-verified-release", baseline_created, true)?;
        assert!(finalize.status.success(), "{}", String::from_utf8_lossy(&finalize.stderr));
        Ok(())
    }

    fn run(&self, name: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_policy(name, false, true)
    }

    fn run_with_policy(
        &self,
        name: &str,
        baseline_created: bool,
        immutable: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        let path = format!("{}:{}", self.root.join("bin").display(), std::env::var("PATH")?);
        let release_id = fs::read_to_string(self.root.join("release.env"))
            .ok()
            .and_then(|contents| {
                contents
                    .lines()
                    .rev()
                    .find_map(|line| line.strip_prefix("RELEASE_ID=").map(str::to_owned))
            });
        let mut command = Command::new(self.root.join("scripts").join(name));
        command
            .current_dir(&self.root)
            .env("PATH", path)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("STAGING_SOURCE_COMMIT", COMMIT)
            .env("ACTIVATION_COMMIT", COMMIT)
            .env("STAGING_RUN_ID", "42")
            .env("RELEASE_TAG", format!("v{}", env!("CARGO_PKG_VERSION")))
            .env("GITHUB_ENV", self.root.join("release.env"))
            .env("BASELINE_CREATED", baseline_created.to_string())
            .env("FIXTURE_IMMUTABLE", immutable.to_string());
        if let Some(release_id) = release_id {
            command.env("RELEASE_ID", release_id);
        }
        Ok(command.output()?)
    }

    fn last_baseline_created(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("release.env"))?
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("BASELINE_CREATED="))
            == Some("true"))
    }

    fn assets(&self) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
        let names = fs::read_dir(self.root.join("remote"))?
            .map(|entry| {
                entry?.file_name().into_string().map_err(|_| std::io::Error::other("asset"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ASSETS.into_iter().chain(["release-baseline.json"]).filter(|name| names.iter().any(|actual| actual == name)).collect())
    }

    fn log(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("log")).unwrap_or_default())
    }

    fn reads(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("reads")).unwrap_or_default())
    }
}
