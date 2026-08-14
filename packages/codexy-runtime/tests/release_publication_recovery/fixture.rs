use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::support::{FixtureCommand as Command, fixture_script_interpreter_path};

#[path = "fixture_materialization.rs"]
mod fixture_materialization;
use fixture_materialization::{bind_scripts, copy_scripts};

pub(crate) const ASSETS: [&str; 3] = [
    "codexy-marketplace-plugin.tar.gz",
    "codexy-runtime-package.tar.gz",
    "runtime-release-receipt.json",
];
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(crate) fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn git_fixture() -> &'static str { r#"#!/bin/sh
case "$1" in fetch|merge-base) exit 0 ;; rev-parse) printf '%s\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;; ls-remote) printf '%s\trefs/tags/v9.9.9\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;; *) exit 1 ;; esac
"# }

pub(crate) fn gh_fixture() -> &'static str { r#"#!/usr/bin/env python3
import hashlib,json,os,pathlib,shutil,sys
root=pathlib.Path.cwd(); remote=root/'remote'; exists=root/'exists'; draft=root/'draft'; log=root/'log'; tag='v9.9.9'; commit='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
def assets():
 return [{'id':i+1,'name':p.name,'size':p.stat().st_size,'digest':'sha256:'+hashlib.sha256(p.read_bytes()).hexdigest()} for i,p in enumerate(sorted(remote.iterdir()))]
def state(api=False):
 s={'id':42,'name':tag,'tag_name':tag,'target_commitish':commit,'draft':draft.read_text().strip()=='true','prerelease':False,'assets':assets()}
 if api: s['immutable']=os.environ.get('FIXTURE_IMMUTABLE','true') == 'true'
 else: s={'id':s['id'],'name':s['name'],'tagName':s['tag_name'],'targetCommitish':s['target_commitish'],'isDraft':s['draft'],'isPrerelease':s['prerelease'],'assets':s['assets']}
 return s
args=sys.argv[1:]
if args[:2]==['release','view']:
 if not exists.exists(): sys.exit(1)
 graph=state(); graph['id']='node-42'; print(json.dumps(graph)); sys.exit()
if args[:2]==['release','create']:
 exists.write_text('yes'); draft.write_text('true'); log.write_text(log.read_text()+'create\n' if log.exists() else 'create\n'); sys.exit()
if args[:2]==['release','download']:
 name=args[args.index('--pattern')+1]; directory=pathlib.Path(args[args.index('--dir')+1]); directory.mkdir(exist_ok=True); target=directory/name
 if target.exists(): sys.exit(1)
 shutil.copy(remote/name,target); sys.exit()
if args[:2]==['release','upload']:
 if draft.read_text().strip() != 'true': sys.exit(1)
 source=pathlib.Path(args[3]); shutil.copy(source,remote/source.name); log.write_text(log.read_text()+'upload '+source.name+'\n' if log.exists() else 'upload '+source.name+'\n'); sys.exit()
if args[:2]==['release','edit']:
 if draft.read_text().strip() != 'true': sys.exit(1)
 draft.write_text('false'); log.write_text(log.read_text()+'publish\n'); sys.exit()
if args and args[0]=='api': print(json.dumps(state(True))); sys.exit()
sys.exit(1)
"# }

pub(crate) struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    git_launcher: PathBuf,
    gh_launcher: PathBuf,
}

impl Fixture {
    pub(crate) fn new(
        existing: &[&str],
        published: bool,
        mismatch: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("release recovery fixture");
        for name in ["bin", "scripts", "dist", "remote"] {
            fs::create_dir_all(root.join(name))?;
        }
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
        copy_scripts(&root)?;
        let git = root.join("bin/git");
        let gh = root.join("bin/gh");
        fs::write(&git, git_fixture())?;
        fs::write(&gh, gh_fixture())?;
        for path in fs::read_dir(root.join("scripts"))?.chain(fs::read_dir(root.join("bin"))?) {
            make_executable(&path?.path())?;
        }
        bind_scripts(&root)?;
        Ok(Self {
            _temp: temp,
            root,
            git_launcher: fixture_script_interpreter_path(&git)?,
            gh_launcher: fixture_script_interpreter_path(&gh)?,
        })
    }

    pub(crate) fn run_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let publish = self.run("publish-verified-release")?;
        assert!(
            publish.status.success(),
            "stdout: {} stderr: {}",
            String::from_utf8_lossy(&publish.stdout),
            String::from_utf8_lossy(&publish.stderr)
        );
        let finalize = self.run_with_settings(
            "finalize-verified-release",
            self.last_baseline_created()?,
            true,
        )?;
        assert!(
            finalize.status.success(),
            "{}",
            String::from_utf8_lossy(&finalize.stderr)
        );
        Ok(())
    }

    pub(crate) fn run(
        &self,
        name: &str,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_settings(name, false, true)
    }

    pub(crate) fn run_with_settings(
        &self,
        name: &str,
        baseline_created: bool,
        settings_allowed: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_policy(name, baseline_created, settings_allowed, true)
    }

    pub(crate) fn run_with_policy(
        &self,
        name: &str,
        baseline_created: bool,
        settings_allowed: bool,
        immutable: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        Ok(Command::new(self.root.join("scripts").join(name))
            .current_dir(&self.root)
            .env_path("FIXTURE_GIT", self.root.join("bin/git"))
            .env_path("FIXTURE_GIT_LAUNCHER", &self.git_launcher)
            .env_path("FIXTURE_GH", self.root.join("bin/gh"))
            .env_path("FIXTURE_GH_LAUNCHER", &self.gh_launcher)
            .env_path(
                "FIXTURE_POSIX_SHELL",
                fixture_script_interpreter_path(&self.root.join("scripts/publish-verified-release"))?,
            )
            .env_path("FIXTURE_SCRIPT_ROOT", &self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("STAGING_SOURCE_COMMIT", COMMIT)
            .env("ACTIVATION_COMMIT", COMMIT)
            .env("STAGING_RUN_ID", "42")
            .env("RELEASE_TAG", "v9.9.9")
            .env("GITHUB_ENV", self.root.join("release.env"))
            .env("BASELINE_CREATED", baseline_created.to_string())
            .env("RELEASE_POLICY_TOKEN", "fixture-token")
            .env("SETTINGS_ALLOWED", settings_allowed.to_string())
            .env("FIXTURE_IMMUTABLE", immutable.to_string())
            .output()?)
    }

    pub(crate) fn last_baseline_created(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("release.env"))?
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("BASELINE_CREATED="))
            == Some("true"))
    }
    pub(crate) fn assets(&self) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
        let names = fs::read_dir(self.root.join("remote"))?
            .map(|entry| {
                entry?
                    .file_name()
                    .into_string()
                    .map_err(|_| std::io::Error::other("asset"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ASSETS
            .into_iter()
            .chain(["release-baseline.json"])
            .filter(|name| names.iter().any(|actual| actual == name))
            .collect())
    }
    pub(crate) fn log(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("log")).unwrap_or_default())
    }
}
