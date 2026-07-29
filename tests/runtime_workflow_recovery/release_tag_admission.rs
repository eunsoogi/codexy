use std::{env, fs, path::Path, path::PathBuf, process::Output};

use serde_yaml::Value;

use crate::support::{self, FixtureCommand as Command};

const STAGING: &str = "0123456789abcdef0123456789abcdef01234567";
const ACTIVATION: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[test]
fn remote_version_tag_admission_is_authoritative_and_race_safe() -> Result<(), Box<dyn std::error::Error>> {
    for state in [RemoteTag::Wrong, RemoteTag::Unpeelable, RemoteTag::Changed, RemoteTag::Appears] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "unsafe {state:?} remote tag unexpectedly admitted");
        assert_eq!(fixture.release_calls()?, 0, "{state:?} reached release creation: {}", String::from_utf8_lossy(&output.stderr));
    }
    for state in [RemoteTag::Exact, RemoteTag::Absent] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "fixture must stop at fake release boundary");
        assert!(String::from_utf8_lossy(&output.stderr).contains("release-create sentinel"));
        assert_eq!(fixture.release_calls()?, 1, "{state:?} tag did not admit release");
        assert_eq!(fixture.show_ref_calls()?, 0, "{state:?} used a local tag snapshot");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum RemoteTag { Wrong, Unpeelable, Changed, Appears, Exact, Absent }

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    script: PathBuf,
    calls: PathBuf,
    show_refs: PathBuf,
}

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
            fs::write(&path, body)?;
            support::make_executable(&path)?;
        }
        let script = root.join("release-step.sh");
        fs::write(&script, format!("#!/bin/sh\nset -e\n{}", release_step()?))?;
        support::make_executable(&script)?;
        fs::write(root.join("remote-state"), remote_state(state))?;
        fs::write(root.join("remote-queries"), "0")?;
        let calls = root.join("release-calls");
        let show_refs = root.join("show-ref-calls");
        Ok(Self { _temp: temp, root, script, calls, show_refs })
    }

    fn run(&self) -> Result<Output, Box<dyn std::error::Error>> {
        let host_path = env::var_os("PATH").ok_or("host PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(env::split_paths(&host_path));
        let mut command = Command::new(&self.script);
        command.current_dir(&self.root);
        command
            .env_path_list("PATH", paths)
            .env_path("REMOTE_STATE", self.root.join("remote-state"))
            .env_path("REMOTE_QUERIES", self.root.join("remote-queries"))
            .env_path("FETCHED_STATE", self.root.join("fetched-state"))
            .env_path("RELEASE_CALLS", &self.calls)
            .env_path("SHOW_REF_CALLS", &self.show_refs)
            .env("STAGING_SOURCE_COMMIT", STAGING)
            .env("ACTIVATION_COMMIT", ACTIVATION)
            .env("STAGING_RUN_ID", "42");
        Ok(command.output()?)
    }

    fn release_calls(&self) -> Result<usize, Box<dyn std::error::Error>> { Ok(lines(&self.calls)?) }
    fn show_ref_calls(&self) -> Result<usize, Box<dyn std::error::Error>> { Ok(lines(&self.show_refs)?) }
}

fn lines(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path).unwrap_or_default().lines().count())
}

fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    publisher["jobs"]["publish-v1-3-0"]["steps"].as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .and_then(|step| step["run"].as_str()).map(str::to_owned).ok_or_else(|| "final release step".into())
}

fn remote_state(state: RemoteTag) -> &'static str {
    match state { RemoteTag::Wrong => "wrong", RemoteTag::Unpeelable => "unpeelable", RemoteTag::Changed => "changed", RemoteTag::Appears => "appears", RemoteTag::Exact => "exact", RemoteTag::Absent => "absent" }
}

fn git_fixture() -> &'static str {
    "#!/bin/sh\nstate() { cat \"$REMOTE_STATE\"; }\nremote_oid() { case \"$1\" in wrong) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; unpeelable) printf '%s\\n' bad-object ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac; }\ncase \"$1\" in\n  fetch) case \"$*\" in *refs/tags/v1.3.0*) value=$(state); [ \"$value\" = changed ] && value=exact; printf '%s\\n' \"$value\" > \"$FETCHED_STATE\" ;; esac ;;\n  ls-remote) count=$(cat \"$REMOTE_QUERIES\"); printf '%s\\n' $((count + 1)) > \"$REMOTE_QUERIES\"; value=$(state); [ \"$value\" = absent ] && exit 0; [ \"$value\" = appears ] && exit 0; [ \"$value\" = changed ] && [ \"$count\" -ge 2 ] && value=wrong; remote_oid \"$value\" | awk '{printf \"%s\\trefs/tags/v1.3.0\\n\", $1}' ;;\n  push) case \"$(state)\" in absent) printf '%s\\n' exact > \"$REMOTE_STATE\" ;; appears) printf '%s\\n' wrong > \"$REMOTE_STATE\"; exit 1 ;; *) exit 91 ;; esac ;;\n  show-ref) printf '%s\\n' local >> \"$SHOW_REF_CALLS\"; exit 1 ;;\n  rev-parse) case \"$*\" in *FETCH_HEAD*) value=$(cat \"$FETCHED_STATE\"); [ \"$value\" = unpeelable ] && exit 1; remote_oid \"$value\" ;; *origin/main*) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; *) printf '%s\\n' \"$2\" ;; esac ;;\n  *) exit 91 ;;\nesac\n"
}

fn jq_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$2\" in .source.stagingSourceCommit) printf '%s\\n' \"$STAGING_SOURCE_COMMIT\" ;; .source.activationCommit) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; .staging.runId) printf '%s\\n' \"$STAGING_RUN_ID\" ;; *) exit 91 ;; esac\n"
}

fn gh_fixture() -> &'static str {
    "#!/bin/sh\nprintf '%s\\n' release >> \"$RELEASE_CALLS\"\nprintf '%s\\n' 'release-create sentinel' >&2\nexit 83\n"
}
