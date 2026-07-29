use std::{env, fs, path::Path, path::PathBuf, process::Output};

use serde_yaml::Value;

use crate::support::{self, FixtureCommand as Command};

const STAGING: &str = "0123456789abcdef0123456789abcdef01234567";
const ACTIVATION: &str = "89abcdef0123456789abcdef0123456789abcdef";

#[test]
fn existing_version_tag_must_match_activation_before_release_creation()
-> Result<(), Box<dyn std::error::Error>> {
    let wrong = Fixture::new(TagState::Wrong)?;
    let output = wrong.run()?;
    assert!(!output.status.success(), "wrong existing tag unexpectedly admitted");
    assert_eq!(
        wrong.release_calls()?,
        0,
        "wrong tag reached release creation: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("existing v1.3.0 tag"));

    let invalid = Fixture::new(TagState::Invalid)?;
    let output = invalid.run()?;
    assert!(!output.status.success(), "unresolvable existing tag unexpectedly admitted");
    assert_eq!(invalid.release_calls()?, 0, "unresolvable tag reached release creation");
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot resolve existing v1.3.0 tag"));

    for state in [TagState::Exact, TagState::Absent] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(!output.status.success(), "fixture must stop at fake release boundary");
        assert!(String::from_utf8_lossy(&output.stderr).contains("release-create sentinel"));
        assert_eq!(fixture.release_calls()?, 1, "{state:?} tag did not admit release");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum TagState {
    Wrong,
    Invalid,
    Exact,
    Absent,
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    script: PathBuf,
    calls: PathBuf,
}

impl Fixture {
    fn new(state: TagState) -> Result<Self, Box<dyn std::error::Error>> {
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
        let calls = root.join("release-calls");
        fs::write(root.join("tag-state"), tag_state(state))?;
        Ok(Self { _temp: temp, root, script, calls })
    }

    fn run(&self) -> Result<Output, Box<dyn std::error::Error>> {
        let host_path = env::var_os("PATH").ok_or("host PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(env::split_paths(&host_path));
        let mut command = Command::new(&self.script);
        command.current_dir(&self.root);
        command
            .env_path_list("PATH", paths)
            .env_path("TAG_STATE", self.root.join("tag-state"))
            .env_path("RELEASE_CALLS", &self.calls)
            .env("STAGING_SOURCE_COMMIT", STAGING)
            .env("ACTIVATION_COMMIT", ACTIVATION)
            .env("STAGING_RUN_ID", "42");
        Ok(command.output()?)
    }

    fn release_calls(&self) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(&self.calls).unwrap_or_default().lines().count())
    }
}

fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .and_then(|step| step["run"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "final release step".into())
}

fn tag_state(state: TagState) -> &'static str {
    match state {
        TagState::Wrong => "ffffffffffffffffffffffffffffffffffffffff",
        TagState::Invalid => "invalid",
        TagState::Exact => ACTIVATION,
        TagState::Absent => "absent",
    }
}

fn git_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$1\" in\n  show-ref) [ \"$(cat \"$TAG_STATE\")\" = absent ] && exit 1; : ;;\n  rev-parse) case \"$*\" in *refs/tags*) tag=$(cat \"$TAG_STATE\"); [ \"$tag\" = invalid ] && exit 1; printf '%s\\n' \"$tag\" ;; *origin/main*) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; *) printf '%s\\n' \"$2\" ;; esac ;;\n  fetch) : ;;\n  *) exit 91 ;;\nesac\n"
}

fn jq_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$2\" in\n  .source.stagingSourceCommit) printf '%s\\n' \"$STAGING_SOURCE_COMMIT\" ;;\n  .source.activationCommit) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;;\n  .staging.runId) printf '%s\\n' \"$STAGING_RUN_ID\" ;;\n  *) exit 91 ;;\nesac\n"
}

fn gh_fixture() -> &'static str {
    "#!/bin/sh\nprintf '%s\\n' release >> \"$RELEASE_CALLS\"\nprintf '%s\\n' 'release-create sentinel' >&2\nexit 83\n"
}
