use std::{fs, io, path::Path, path::PathBuf, process::Output};

use serde_yaml::Value;

use crate::support::{
    self, FixtureCommand as Command, write_posix_fixture_command,
    write_posix_fixture_shell_runner_with_scrub,
};

const STAGING: &str = "0123456789abcdef0123456789abcdef01234567";
const ACTIVATION: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[rustfmt::skip]
fn contextual_error(stage: &str, path: &Path, details: &str, error: io::Error) -> io::Error { let raw_os_error = error.raw_os_error(); io::Error::new(error.kind(), format!("{stage}: path={} {details} raw_os_error={raw_os_error:?}: {error}", path.display())) }
macro_rules! fixture_io {
    ($stage:expr, $path:expr, $result:expr) => {
        $result.map_err(|error| contextual_error($stage, $path, "", error))?
    };
}
#[rustfmt::skip]
fn fixture_output(command: &mut Command, path: &Path, cwd: &Path) -> io::Result<Output> { let program = command.get_program().to_string_lossy().into_owned(); let argv = std::iter::once(program.clone()).chain(command.get_args().map(|arg| arg.to_string_lossy().into_owned())).collect::<Vec<_>>().join(" "); command.output().map_err(|error| contextual_error("spawn/output fixture command", path, &format!("executable={program} cwd={} argv=[{argv}]", cwd.display()), error)) }
#[rustfmt::skip]
fn assert_fixture_error_context(path: &Path, cwd: &Path, raw_os_error: bool) -> Result<(), Box<dyn std::error::Error>> { let mut command = Command::new(path); command.current_dir(cwd); let text = fixture_output(&mut command, path, cwd).expect_err("fixture error").to_string(); let (prefix, fields) = text.split_once("path=").ok_or("path field")?; assert_eq!(prefix, "spawn/output fixture command: "); let (actual_path, fields) = fields.split_once(" executable=").ok_or("executable field")?; assert_eq!(actual_path, path.to_str().ok_or("fixture path")?); let (actual_executable, fields) = fields.split_once(" cwd=").ok_or("cwd field")?; let (actual_cwd, fields) = fields.split_once(" argv=[").ok_or("argv field")?; let (actual_argv, fields) = fields.split_once("] raw_os_error=").ok_or("raw error field")?; let (actual_raw, _) = fields.split_once(": ").ok_or("error detail")?; assert_eq!(actual_cwd, cwd.to_str().ok_or("cwd path")?); assert_eq!(actual_argv, actual_executable); if raw_os_error { assert_ne!(actual_raw, "None"); } Ok(()) }
#[test]
#[rustfmt::skip]
fn fixture_error_context_names_missing_fixture() -> Result<(), Box<dyn std::error::Error>> { let temp = tempfile::tempdir()?; let path = temp.path().join("missing fixture.sh"); assert_fixture_error_context(&path, temp.path(), false) }
#[cfg(unix)]
#[test]
#[rustfmt::skip]
fn fixture_error_context_names_non_executable_fixture() -> Result<(), Box<dyn std::error::Error>> { let temp = tempfile::tempdir()?; let path = temp.path().join("non-executable fixture.sh"); fs::write(&path, "#!/bin/sh\nexit 0\n")?; assert_fixture_error_context(&path, temp.path(), true) }
#[test]
fn remote_version_tag_admission_uses_authenticated_create_only_api() -> Result<(), Box<dyn std::error::Error>> {
    for state in [
        RemoteTag::Wrong,
        RemoteTag::Unpeelable,
        RemoteTag::Changed,
        RemoteTag::ExactOutsideProtectedMain,
        RemoteTag::ExactLosesProtectedMainAfterSource,
        RemoteTag::AbsentAfterMainAdvance,
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
    for state in [RemoteTag::Exact, RemoteTag::ExactAfterMainAdvance, RemoteTag::Absent, RemoteTag::ConcurrentExact] {
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
    assert_eq!(fixture.command_calls("git")?, 13, "host git fallthrough");
    assert_eq!(fixture.command_calls("jq")?, 3, "host jq fallthrough");
    assert_eq!(fixture.command_calls("gh")?, 1, "host gh fallthrough");
    Ok(())
}

#[test]
fn fixture_discards_every_inherited_git_and_github_state()
-> Result<(), Box<dyn std::error::Error>> {
    assert_inherited_state_discarded(&[
        ("GIT_DIR", "host-git-dir"),
        ("GIT_WORK_TREE", "host-work-tree"),
        ("GIT_INDEX_FILE", "host-index"),
        ("GIT_COMMON_DIR", "host-common"),
        ("GH_CONFIG_DIR", "host-gh-config"),
        ("GH_HOST", "host-gh"),
        ("GH_ENTERPRISE_TOKEN", "host-enterprise-token"),
        ("GH_TOKEN", "host-gh-token"),
        ("GITHUB_TOKEN", "host-token"),
    ])
}

fn assert_inherited_state_discarded(
    poison: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
    let output = fixture.run_with_inherited_state(poison)?;
    assert!(!output.status.success(), "inherited state admitted concurrent wrong tag");
    assert_eq!(fixture.api_calls()?, 1, "inherited state blocked authenticated API");
    assert_eq!(fixture.release_calls()?, 0, "inherited state reached release");
    assert_eq!(fixture.git_push_calls()?, 0, "inherited state used git push");
    assert_eq!(fixture.command_calls("git")?, 13, "inherited Git state leaked");
    assert_eq!(fixture.command_calls("jq")?, 3, "inherited state leaked into jq");
    assert_eq!(fixture.command_calls("gh")?, 1, "inherited state leaked");
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum RemoteTag { Wrong, Unpeelable, Changed, Exact, ExactAfterMainAdvance, ExactOutsideProtectedMain, ExactLosesProtectedMainAfterSource, AbsentAfterMainAdvance, Absent, ConcurrentExact, ConcurrentWrong, ConcurrentUnpeelable, ApiAuth, ApiFailure }

impl RemoteTag {
    fn create_api_calls(self) -> usize {
        usize::from(!matches!(self, Self::Wrong | Self::Unpeelable | Self::Changed | Self::Exact | Self::ExactAfterMainAdvance | Self::ExactOutsideProtectedMain | Self::ExactLosesProtectedMainAfterSource | Self::AbsentAfterMainAdvance))
    }
}

struct Fixture { _temp: tempfile::TempDir, root: PathBuf, script: PathBuf, runner: PathBuf, calls: PathBuf, pushes: PathBuf, api_calls: PathBuf, merge_base_calls: PathBuf }

impl Fixture {
    fn new(state: RemoteTag) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = fixture_io!("create fixture tempdir", Path::new("<tempdir>"), tempfile::tempdir());
        let root = temp.path().join("release tag fixture with spaces");
        let bin = root.join("bin");
        fixture_io!("create fixture dist directory", &root.join("dist"), fs::create_dir_all(root.join("dist")));
        fixture_io!("create fixture bin directory", &bin, fs::create_dir(&bin));
        fixture_io!(
            "write fixture release receipt",
            &root.join("dist/runtime-release-receipt.json"),
            fs::write(root.join("dist/runtime-release-receipt.json"), "{}")
        );
        for (name, body) in [("git", git_fixture()), ("jq", jq_fixture()), ("gh", gh_fixture())] {
            let path = bin.join(name);
            fixture_io!(&format!("write fixture command {name}"), &path, write_posix_fixture_command(&path, body));
        }
        let script = root.join("release-step.sh");
        fixture_io!(
            "write release-step fixture",
            &script,
            fs::write(&script, format!("#!/bin/sh\nset -e\n{}", release_step()?))
        );
        fixture_io!("chmod release-step fixture", &script, support::make_executable(&script));
        let runner = root.join("bound-release-step.sh");
        fixture_io!(
            "write bound runner; sh -n validation and chmod",
            &runner,
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
            )
        );
        fixture_io!("write remote state", &root.join("remote-state"), fs::write(root.join("remote-state"), remote_state(state)));
        fixture_io!("write remote query state", &root.join("remote-queries"), fs::write(root.join("remote-queries"), "0"));
        let calls = root.join("release-calls");
        let pushes = root.join("git-push-calls");
        let api_calls = root.join("api-calls");
        let merge_base_calls = root.join("merge-base-calls");
        Ok(Self { _temp: temp, root, script, runner, calls, pushes, api_calls, merge_base_calls })
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
            .env_path("MERGE_BASE_CALLS", &self.merge_base_calls)
            .env_path("CODEXY_FIXTURE_COMMAND_TRACE", self.root.join("command-trace"))
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("CODEXY_FIXTURE_GH_TOKEN", "fixture-token")
            .env("STAGING_SOURCE_COMMIT", STAGING)
            .env("ACTIVATION_COMMIT", ACTIVATION)
            .env("STAGING_RUN_ID", "42");
        Ok(fixture_output(&mut command, &self.runner, &self.root)?)
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
    let steps = publisher["jobs"]["publish-v1-3-0"]["steps"].as_sequence().ok_or("release steps")?;
    let source = steps.iter().find(|step| step["name"] == "Verify selected protected-main source")
        .and_then(|step| step["run"].as_str()).ok_or("protected main source")?;
    let release = steps.iter().find(|step| step["name"] == "Create and verify the only public version release")
        .and_then(|step| step["run"].as_str()).ok_or("final release step")?;
    Ok(format!("{source}\n{release}"))
}

fn remote_state(state: RemoteTag) -> &'static str {
    match state {
        RemoteTag::Wrong => "wrong", RemoteTag::Unpeelable => "unpeelable", RemoteTag::Changed => "changed",
        RemoteTag::Exact => "exact", RemoteTag::ExactAfterMainAdvance => "exact-after-main-advance", RemoteTag::ExactOutsideProtectedMain => "exact-outside-protected-main", RemoteTag::ExactLosesProtectedMainAfterSource => "exact-loses-protected-main-after-source", RemoteTag::AbsentAfterMainAdvance => "absent-after-main-advance", RemoteTag::Absent => "absent", RemoteTag::ConcurrentExact => "concurrent-exact",
        RemoteTag::ConcurrentWrong => "concurrent-wrong", RemoteTag::ConcurrentUnpeelable => "concurrent-unpeelable",
        RemoteTag::ApiAuth => "api-auth", RemoteTag::ApiFailure => "api-failure",
    }
}

fn git_fixture() -> &'static str {
    "#!/bin/sh\nif test -n \"${GIT_DIR+x}${GIT_WORK_TREE+x}${GIT_INDEX_FILE+x}${GIT_COMMON_DIR+x}\"; then printf '%s\\n' 'inherited Git state reached fixture' >&2; exit 92; fi\nstate() { cat \"$REMOTE_STATE\"; }\nremote_oid() { case \"$1\" in wrong) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; unpeelable) printf '%s\\n' bad-object ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac; }\ncase \"$1\" in\n  fetch) case \"$*\" in *refs/tags/v1.3.0*|*--tags*) value=$(state); [ \"$value\" = changed ] && value=exact; printf '%s\\n' \"$value\" > \"$FETCHED_STATE\" ;; esac ;;\n  ls-remote) count=$(cat \"$REMOTE_QUERIES\"); printf '%s\\n' $((count + 1)) > \"$REMOTE_QUERIES\"; value=$(state); case \"$value\" in absent|absent-after-main-advance|concurrent-exact|concurrent-wrong|concurrent-unpeelable|api-auth|api-failure) exit 0 ;; changed) [ \"$count\" -ge 2 ] && value=wrong ;; esac; remote_oid \"$value\" | awk '{printf \"%s\\trefs/tags/v1.3.0\\n\", $1}' ;;\n  push) printf '%s\\n' push >> \"$GIT_PUSH_CALLS\"; exit 91 ;;\n  checkout) : ;;\n  merge-base) if [ \"$2\" = --is-ancestor ] && [ \"$3\" = \"$STAGING_SOURCE_COMMIT\" ] && [ \"$4\" = \"$ACTIVATION_COMMIT\" ]; then :; elif [ \"$2\" = --is-ancestor ] && [ \"$3\" = \"$ACTIVATION_COMMIT\" ] && [ \"$4\" = origin/main ]; then calls=$(cat \"$MERGE_BASE_CALLS\" 2>/dev/null || printf 0); printf '%s\\n' $((calls + 1)) > \"$MERGE_BASE_CALLS\"; case \"$(state)\" in exact-outside-protected-main) exit 1 ;; exact-loses-protected-main-after-source) test \"$calls\" -eq 0 ;; esac; else exit 91; fi ;;\n  rev-parse) case \"$*\" in *FETCH_HEAD*|*refs/tags/v1.3.0*) value=$(cat \"$FETCHED_STATE\"); [ \"$value\" = unpeelable ] && exit 1; remote_oid \"$value\" ;; *origin/main*) case \"$(state)\" in exact-after-main-advance|absent-after-main-advance) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac ;; *) printf '%s\\n' \"$2\" ;; esac ;;\n  *) exit 91 ;;\nesac\n"
}

fn jq_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$2\" in .source.stagingSourceCommit) printf '%s\\n' \"$STAGING_SOURCE_COMMIT\" ;; .source.activationCommit) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; .staging.runId) printf '%s\\n' \"$STAGING_RUN_ID\" ;; *) exit 91 ;; esac\n"
}

fn gh_fixture() -> &'static str {
    "#!/bin/sh\nif test -n \"${GH_CONFIG_DIR+x}${GH_HOST+x}${GH_ENTERPRISE_TOKEN+x}${GITHUB_TOKEN+x}\"; then printf '%s\\n' 'inherited GitHub state reached fixture' >&2; exit 92; fi\nstate() { cat \"$REMOTE_STATE\"; }\nif [ \"$1\" = api ]; then\n  printf '%s\\n' api >> \"$API_CALLS\"\n  [ \"$GH_TOKEN\" = fixture-token ] || { printf '%s\\n' 'HTTP/2.0 401 Unauthorized'; exit 1; }\n  case \"$(state)\" in absent) printf '%s\\n' exact > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 201 Created'; exit 0 ;; concurrent-exact) printf '%s\\n' exact > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; concurrent-wrong) printf '%s\\n' wrong > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; concurrent-unpeelable) printf '%s\\n' unpeelable > \"$REMOTE_STATE\"; printf '%s\\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;; api-auth) printf '%s\\n' 'HTTP/2.0 401 Unauthorized'; exit 1 ;; api-failure) printf '%s\\n' 'HTTP/2.0 500 Server Error'; exit 1 ;; *) exit 91 ;; esac\nfi\nif [ \"$1 $2\" = 'release view' ]; then exit 1; fi\nprintf '%s\\n' release >> \"$RELEASE_CALLS\"\nprintf '%s\\n' 'release-create sentinel' >&2\nexit 83\n"
}
