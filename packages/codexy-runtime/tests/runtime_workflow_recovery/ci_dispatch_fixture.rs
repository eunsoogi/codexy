use serde_json::{Value, json};
use std::{
    fs,
    process::{Command, Output},
};

pub(super) const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const BASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const BRANCH: &str = "codexy/runtime-activation-v1.7.0-staging-42-1";
pub(super) const WORKFLOWS: [&str; 5] = [
    "rust-test.yml",
    "language-lint.yml",
    "touched-loc-gate.yml",
    "plugin-runtime-binaries.yml",
    "python-package.yml",
];

pub(super) struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let fixture = Self { root };
        let mut state = json!({});
        for (index, workflow) in WORKFLOWS.iter().enumerate() {
            let title = if *workflow == "touched-loc-gate.yml" {
                format!("CI {HEAD} base {BASE}")
            } else {
                format!("CI {HEAD}")
            };
            state[*workflow] = json!([{
                "databaseId": index + 100, "event": "pull_request",
                "headSha": HEAD, "headBranch": BRANCH,
                "displayTitle": title, "status": "completed", "conclusion": "success",
            }]);
        }
        fs::write(
            fixture.root.path().join("state.json"),
            serde_json::to_vec(&state)?,
        )?;
        fs::write(fixture.root.path().join("gh.py"), GH)?;
        Ok(fixture)
    }

    pub(super) fn change(
        &self,
        workflow: &str,
        update: impl FnOnce(&mut Value),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.root.path().join("state.json");
        let mut state: Value = serde_json::from_slice(&fs::read(&path)?)?;
        update(&mut state[workflow]);
        fs::write(path, serde_json::to_vec(&state)?)?;
        Ok(())
    }

    pub(super) fn run(&self) -> Result<Output, Box<dyn std::error::Error>> {
        let workflow = super::super::workflow("runtime-activation.yml")?;
        let dispatch = super::super::run(
            &workflow,
            "open-activation-pr",
            "Dispatch required CI for exact activation head",
        )?;
        Ok(Command::new("bash")
            .arg("-c")
            .arg(format!("{PRELUDE}\n{dispatch}"))
            .env("FIXTURE_ROOT", self.root.path())
            .env("RUNNER_TEMP", self.root.path())
            .env("BOOTSTRAP_VERSION", "1.7.0")
            .env("GH_REPO", "eunsoogi/codexy")
            .env("FIXTURE_HEAD", HEAD)
            .env("FIXTURE_BASE", BASE)
            .env("FIXTURE_BRANCH", BRANCH)
            .output()?)
    }

    pub(super) fn dispatched(&self) -> Result<Vec<String>, std::io::Error> {
        let path = self.root.path().join("dispatches");
        if !path.exists() {
            return Ok(vec![]);
        }
        Ok(fs::read_to_string(path)?
            .lines()
            .map(str::to_owned)
            .collect())
    }
}

const PRELUDE: &str = r#"
printf '%s\n' "$FIXTURE_BRANCH" > "$RUNNER_TEMP/codexy-runtime-activation-branch"
git() {
  test -f "$RUNNER_TEMP/codexy-runtime-activation-branch"
  case "$*" in
    'rev-parse HEAD') printf '%s\n' "$FIXTURE_HEAD" ;;
    'rev-parse '*'^'{commit}) printf '%s\n' "$FIXTURE_BASE" ;;
    'check-ref-format '*) ;;
    'ls-remote '*) printf '%s\trefs/heads/%s\n' "$FIXTURE_HEAD" "$FIXTURE_BRANCH" ;;
    *) echo "unexpected git call: $*" >&2; return 1 ;;
  esac
}
gh() { python3 "$FIXTURE_ROOT/gh.py" "$@"; }
timeout() { shift; "$@"; }
"#;

const GH: &str = r#"
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
root = Path(os.environ['FIXTURE_ROOT'])
state_path = root / 'state.json'
state = json.loads(state_path.read_text())
head, base = os.environ['FIXTURE_HEAD'], os.environ['FIXTURE_BASE']
branch = os.environ['FIXTURE_BRANCH']
def value(flag):
    return args[args.index(flag) + 1]
def save():
    state_path.write_text(json.dumps(state))
if args[:2] == ['pr', 'list']:
    print('1005')
elif args[:2] == ['pr', 'view']:
    print(base if value('--json') == 'baseRefOid' else head)
elif args[0] == 'api':
    if '--repo' in args:
        sys.exit('unknown flag: --repo')
    print('plugins/codexy/skills/wiki/SKILL.md')
elif args[:2] == ['run', 'list']:
    print(json.dumps(state[value('--workflow')]))
elif args[:2] == ['workflow', 'run']:
    workflow = args[2]
    assert value('--ref') == branch
    assert f'head_sha={head}' in args
    if workflow == 'rust-test.yml':
        assert 'run_mode=ci' in args
    title = f'CI {head}'
    if workflow == 'touched-loc-gate.yml':
        assert f'base_sha={base}' in args
        title += f' base {base}'
    with (root / 'dispatches').open('a') as output:
        output.write(workflow + '\n')
    state[workflow].append(dict(databaseId=1000+list(state).index(workflow),
        event='workflow_dispatch', headSha=head,
        headBranch=branch, displayTitle=title,
        status='completed', conclusion='success'))
    save()
elif args[:2] in (['run', 'view'], ['run', 'watch']):
    run = next(run for runs in state.values() for run in runs if run['databaseId'] == int(args[2]))
    if args[1] == 'watch':
        run.update(status='completed', conclusion='success')
        save()
    else:
        print(json.dumps(run))
else:
    sys.exit('unexpected gh arguments: ' + repr(args))
"#;
