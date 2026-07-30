use std::{fs, path::Path, path::PathBuf, process::Output};

use serde_yaml::Value;

use crate::support::{
    self, FixtureCommand as Command, write_posix_fixture_command,
    write_posix_fixture_shell_runner_with_scrub,
};

const STAGING: &str = "0123456789abcdef0123456789abcdef01234567";
const ACTIVATION: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[test]
fn remote_version_tag_admission_uses_authenticated_create_only_api() -> Result<(), Box<dyn std::error::Error>> {
    for state in [
        RemoteTag::Wrong,
        RemoteTag::Unpeelable,
        RemoteTag::Changed,
        RemoteTag::ConcurrentWrong,
        RemoteTag::ConcurrentUnpeelable,
        RemoteTag::ApiAuth,
        RemoteTag::ApiFailure,
    ] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "unsafe {state:?} tag unexpectedly admitted");
        assert_eq!(fixture.release_calls()?, 0, "{state:?} reached release creation: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(fixture.git_push_calls()?, 0, "{state:?} used unauthenticated git push");
        assert_eq!(fixture.api_calls()?, state.create_api_calls(), "{state:?} API admission count");
    }
    for state in [RemoteTag::Exact, RemoteTag::Absent, RemoteTag::ConcurrentExact] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "fixture must stop at fake release boundary");
        assert!(String::from_utf8_lossy(&output.stderr).contains("release-create sentinel"));
        assert_eq!(fixture.release_calls()?, 1, "{state:?} tag did not admit release");
        assert_eq!(fixture.git_push_calls()?, 0, "{state:?} used unauthenticated git push");
        assert_eq!(fixture.api_calls()?, state.create_api_calls(), "{state:?} API admission count");
    }
    Ok(())
}

#[test]
fn concurrent_wrong_uses_only_fixture_commands_before_rejection()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
    let output = fixture.run()?;
    assert!(!output.status.success(), "concurrent wrong tag unexpectedly admitted");
    assert_eq!(fixture.api_calls()?, 1, "authenticated API was not called");
    assert_eq!(fixture.remote_state()?, "wrong", "API did not set wrong remote ref");
    assert_eq!(fixture.release_calls()?, 0, "wrong tag reached release creation");
    assert_eq!(fixture.command_calls("git")?, 6, "host git fallthrough");
    assert_eq!(fixture.command_calls("jq")?, 3, "host jq fallthrough");
    assert_eq!(fixture.command_calls("gh")?, 1, "host gh fallthrough");
    Ok(())
}

#[test]
fn fixture_discards_inherited_git_and_gh_state() -> Result<(), Box<dyn std::error::Error>> {
    const POISONED: [(&str, &str); 9] = [
        ("GIT_DIR", "host-git-dir"),
        ("GIT_WORK_TREE", "host-work-tree"),
        ("GIT_INDEX_FILE", "host-index"),
        ("GIT_COMMON_DIR", "host-common"),
        ("GH_CONFIG_DIR", "host-gh-config"),
        ("GH_HOST", "host-gh"),
        ("GH_ENTERPRISE_TOKEN", "host-enterprise-token"),
        ("GH_TOKEN", "host-gh-token"),
        ("GITHUB_TOKEN", "host-token"),
    ];
    for poison in POISONED {
        let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
        let output = fixture.run_with_inherited_state(&[poison])?;
        assert!(!output.status.success(), "{0} admitted concurrent wrong tag", poison.0);
        assert_eq!(fixture.api_calls()?, 1, "{0} blocked authenticated API", poison.0);
        assert_eq!(fixture.release_calls()?, 0, "{0} reached release", poison.0);
        assert_eq!(fixture.git_push_calls()?, 0, "{0} used git push", poison.0);
        assert_eq!(fixture.command_calls("git")?, 6, "{0} leaked host git", poison.0);
        assert_eq!(fixture.command_calls("jq")?, 3, "{0} leaked host jq", poison.0);
        assert_eq!(fixture.command_calls("gh")?, 1, "{0} leaked host gh", poison.0);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum RemoteTag { Wrong, Unpeelable, Changed, Exact, Absent, ConcurrentExact, ConcurrentWrong, ConcurrentUnpeelable, ApiAuth, ApiFailure }

impl RemoteTag {
    fn create_api_calls(self) -> usize {
        usize::from(!matches!(self, Self::Wrong | Self::Unpeelable | Self::Changed | Self::Exact))
    }
}

struct Fixture { _temp: tempfile::TempDir, root: PathBuf, script: PathBuf, runner: PathBuf, calls: PathBuf, pushes: PathBuf, api_calls: PathBuf }

impl Fixture {
    fn new(state: RemoteTag) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("release tag fixture with spaces");
        let bin = root.join("bin");
        fs::create_dir_all(root.join("dist"))?;
        fs::create_dir(&bin)?;
        fs::write(root.join("dist/runtime-release-receipt.json"), "{}")?;
        for (name, body) in [("git", git_fixture()), ("jq", jq_fixture()), ("gh", gh_fixture())] {
            let path = bin.join(name);
            write_posix_fixture_command(&path, body)?;
        }
        let script = root.join("release-step.sh");
        fs::write(&script, format!("#!/bin/sh\nset -e\n{}", release_step()?))?;
        support::make_executable(&script)?;
        let runner = root.join("bound-release-step.sh");
        write_posix_fixture_shell_runner_with_scrub(
            &runner,
            "CODEXY_FIXTURE_RELEASE_STEP",
            &[
                ("git", "CODEXY_FIXTURE_GIT"),
                ("jq", "CODEXY_FIXTURE_JQ"),
                ("gh", "CODEXY_FIXTURE_GH"),
            ],
            &[
                "GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR", "GH_CONFIG_DIR",
                "GH_HOST", "GH_ENTERPRISE_TOKEN", "GH_TOKEN", "GITHUB_TOKEN",
            ],
            &[("GH_TOKEN", "CODEXY_FIXTURE_GH_TOKEN")],
        )?;
        fs::write(root.join("remote-state"), remote_state(state))?;
        fs::write(root.join("remote-queries"), "0")?;
        let calls = root.join("release-calls");
        let pushes = root.join("git-push-calls");
        let api_calls = root.join("api-calls");
        Ok(Self { _temp: temp, root, script, runner, calls, pushes, api_calls })
    }

    fn run(&self) -> Result<Output, Box<dyn std::error::Error>> {
        self.run_with_inherited_state(&[])
    }

    fn run_with_inherited_state(
        &self,
        inherited: &[(&str, &str)],
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let mut command = Command::new(&self.runner);
        command.current_dir(&self.root);
        for (key, value) in inherited {
            command.env(key, value);
        }
        command
            .env_path("CODEXY_FIXTURE_RELEASE_STEP", &self.script)
            .env_path("CODEXY_FIXTURE_GIT", self.root.join("bin/git"))
            .env_path("CODEXY_FIXTURE_JQ", self.root.join("bin/jq"))
            .env_path("CODEXY_FIXTURE_GH", self.root.join("bin/gh"))
            .env_path("REMOTE_STATE", self.root.join("remote-state"))
            .env_path("REMOTE_QUERIES", self.root.join("remote-queries"))
            .env_path("FETCHED_STATE", self.root.join("fetched-state"))
            .env_path("RELEASE_CALLS", &self.calls)
            .env_path("GIT_PUSH_CALLS", &self.pushes)
            .env_path("API_CALLS", &self.api_calls)
            .env_path("CODEXY_FIXTURE_COMMAND_TRACE", self.root.join("command-trace"))
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("CODEXY_FIXTURE_GH_TOKEN", "fixture-token")
            .env("STAGING_SOURCE_COMMIT", STAGING)
            .env("ACTIVATION_COMMIT", ACTIVATION)
            .env("STAGING_RUN_ID", "42");
        Ok(command.output()?)
    }

    fn release_calls(&self) -> Result<usize, Box<dyn std::error::Error>> { lines(&self.calls) }
    fn git_push_calls(&self) -> Result<usize, Box<dyn std::error::Error>> { lines(&self.pushes) }
    fn api_calls(&self) -> Result<usize, Box<dyn std::error::Error>> { lines(&self.api_calls) }
    fn command_calls(&self, name: &str) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("command-trace"))?
            .lines().filter(|line| *line == name).count())
    }
    fn remote_state(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("remote-state"))?.trim().to_owned())
    }
}

fn lines(path: &Path) -> Result<usize, Box<dyn std::error::Error>> { Ok(fs::read_to_string(path).unwrap_or_default().lines().count()) }

fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    publisher["jobs"]["publish-v1-3-0"]["steps"].as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .and_then(|step| step["run"].as_str()).map(str::to_owned).ok_or_else(|| "final release step".into())
}

fn remote_state(state: RemoteTag) -> &'static str {
    match state {
        RemoteTag::Wrong => "wrong", RemoteTag::Unpeelable => "unpeelable", RemoteTag::Changed => "changed",
        RemoteTag::Exact => "exact", RemoteTag::Absent => "absent", RemoteTag::ConcurrentExact => "concurrent-exact",
        RemoteTag::ConcurrentWrong => "concurrent-wrong", RemoteTag::ConcurrentUnpeelable => "concurrent-unpeelable",
        RemoteTag::ApiAuth => "api-auth", RemoteTag::ApiFailure => "api-failure",
    }
}

fn git_fixture() -> &'static str {
    "#!/bin/sh\nif test -n \"${GIT_DIR+x}${GIT_WORK_TREE+x}${GIT_INDEX_FILE+x}${GIT_COMMON_DIR+x}\"; then printf '%s\\n' 'inherited Git state reached fixture' >&2; exit 92; fi\nstate() { cat \"$REMOTE_STATE\"; }\nremote_oid() { case \"$1\" in wrong) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; unpeelable) printf '%s\\n' bad-object ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac; }\ncase \"$1\" in\n  fetch) case \"$*\" in *refs/tags/v1.3.0*) value=$(state); [ \"$value\" = changed ] && value=exact; printf '%s\\n' \"$value\" > \"$FETCHED_STATE\" ;; esac ;;\n  ls-remote) count=$(cat \"$REMOTE_QUERIES\"); printf '%s\\n' $((count + 1)) > \"$REMOTE_QUERIES\"; value=$(state); case \"$value\" in absent|concurrent-exact|concurrent-wrong|concurrent-unpeelable|api-auth|api-failure) exit 0 ;; changed) [ \"$count\" -ge 2 ] && value=wrong ;; esac; remote_oid \"$value\" | awk '{printf \"%s\\trefs/tags/v1.3.0\\n\", $1}' ;;\n  push) printf '%s\\n' push >> \"$GIT_PUSH_CALLS\"; exit 91 ;;\n  rev-parse) case \"$*\" in *FETCH_HEAD*) value=$(cat \"$FETCHED_STATE\"); [ \"$value\" = unpeelable ] && exit 1; remote_oid \"$value\" ;; *origin/main*) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; *) printf '%s\\n' \"$2\" ;; esac ;;\n  *) exit 91 ;;\nesac\n"
}

fn jq_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$2\" in .source.stagingSourceCommit) printf '%s\\n' \"$STAGING_SOURCE_COMMIT\" ;; .source.activationCommit) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; .staging.runId) printf '%s\\n' \"$STAGING_RUN_ID\" ;; *) exit 91 ;; esac\n"
}

fn gh_fixture() -> &'static str {
    "#!/bin/sh\nif test -n \"${GH_CONFIG_DIR+x}${GH_HOST+x}${GH_ENTERPRISE_TOKEN+x}${GITHUB_TOKEN+x}\"; then printf '%s\\n' 'inherited GitHub state reached fixture' >&2; exit 92; fi\nstate() { cat \"$REMOTE_STATE\"; }\nif [ \"$1\" = api ]; then\n  printf '%s\\n' api >> \"$API_CALLS\"\n  [ \"$GH_TOKEN\" = fixture-token ] || { printf '%s\\n' 'HTTP/2.0 401 Unauthorized'; exit 1; }\n  case \"$(state)\" in absent) printf '%s\\n' exact > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 201 Created'; exit 0 ;; concurrent-exact) printf '%s\\n' exact > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; concurrent-wrong) printf '%s\\n' wrong > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; concurrent-unpeelable) printf '%s\\n' unpeelable > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; api-auth) printf '%s\\n' 'HTTP/2.0 401 Unauthorized'; exit 1 ;; api-failure) printf '%s\\n' 'HTTP/2.0 500 Server Error'; exit 1 ;; *) exit 91 ;; esac\nfi\nprintf '%s\\n' release >> \"$RELEASE_CALLS\"\nprintf '%s\\n' 'release-create sentinel' >&2\nexit 83\n"
}
