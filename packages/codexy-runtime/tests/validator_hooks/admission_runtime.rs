use std::path::{Path, PathBuf};
use crate::support::{FixtureCommand as Command, hook_fixture_model_input};

use serde_json::{Value, json};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "admission_runtime/concern_launchers.rs"]
mod concern_launchers;

#[test]
fn opaque_graphql_mutations_fail_closed_without_blocking_queries() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, "gh api graphql -f query='mutation { mergePullRequest(input:{pullRequestId:\"PR_owned_node\",mergeMethod:MERGE}) { pullRequest { id } } }'", true, &[])?;
    assert_case(&root, &foreign, "gh api graphql -f owner=openai -f name=codex -f query='mutation { mergePullRequest(input:{pullRequestId:\"PR_owned_node\",mergeMethod:MERGE}) { pullRequest { id } } }'", true, &[])
}

#[test]
fn graphql_comments_and_strings_remain_read_only_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for command in [
        "gh api graphql -f query='query { viewer { login } }'",
        "gh api graphql -f query='query { viewer { login } }\n# mutation { mergePullRequest }'",
        "gh api graphql -f query='query { search(query:\"mutation { mergePullRequest }\",type:ISSUE,first:1) { issueCount } }'",
        "gh api graphql -f query='query mutation { viewer { login } }'",
        "gh api graphql -f query='{ mutation: viewer { login } }'",
        "gh api graphql -f query='query { ...mutation } fragment mutation on Query { viewer { login } }'",
    ] { assert_case(&root, &foreign, command, false, &[])?; }
    Ok(())
}

#[test]
fn malformed_graphql_escapes_fail_closed() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, r#"gh api graphql -f query='query { search(query:\"bad\q\",type:ISSUE,first:1) { issueCount } }'"#, true, &[])
}

#[test]
fn hash_path_aliases_cannot_disguise_git_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    assert_case(&root, &owned, "hash -p /usr/bin/git safe; safe push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "hash -t git; git status --short", false, &[])
}

#[test]
fn inherited_git_common_dir_fails_closed_for_mutations_only() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let environment = [("GIT_DIR", foreign.join(".git")), ("GIT_COMMON_DIR", owned.join(".git"))];
    let environment = environment.iter().map(|(name, value)| (*name, value.as_os_str())).collect::<Vec<_>>();
    assert_case(&root, &foreign, "git push --force origin topic", true, &environment)?;
    assert_case(&root, &foreign, "git status --short", false, &environment)
}

#[test]
fn thread_delivery_requires_nonempty_model_and_thinking() -> TestResult {
    let root = plugin_root();
    assert_tool_case(
        &root,
        "codex_app__send_message_to_thread",
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":"medium"}),
        false,
    )?;
    for input in [
        json!({"threadId":"parent","thinking":"medium"}),
        json!({"threadId":"parent","model":null,"thinking":"medium"}),
        json!({"threadId":"parent","model":"","thinking":"medium"}),
        json!({"threadId":"parent","model":"gpt-5.6-sol"}),
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":null}),
        json!({"threadId":"parent","model":"gpt-5.6-sol","thinking":""}),
    ] {
        assert_tool_case(&root, "codex_app__send_message_to_thread", input, true)?;
    }
    Ok(())
}

#[test]
fn ordinary_launcher_variables_do_not_make_unrelated_commands_opaque() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, "printf '%s\\n' \"$HOME:$PATH:$USER\"", false, &[])?;
    assert_case(&root, &foreign, "printf '%s\\n' \"$UNKNOWN_RUNTIME_VALUE\"", true, &[])
}

#[test]
fn native_windows_drive_paths_preserve_their_anchor_in_the_policy_model() -> TestResult {
    let root = plugin_root();
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python)
        .arg("-c")
        .arg(
            r#"import ntpath
from pathlib import PureWindowsPath
import codexy_policy.filesystem_state as filesystem_state
import codexy_policy.executable_identity as executable_identity

filesystem_state.os.path = ntpath
filesystem_state.Path = PureWindowsPath
filesystem_state.state = lambda value, paths: dict(paths).get(value, filesystem_state.ABSENT)
cwd = r"C:\work\owned"
source = r"C:\Program Files\Git\usr\bin\printf.exe"
assert filesystem_state.resolved_location(source, cwd, ()) == source
mkdir = filesystem_state._mkdir_trace(r"C:\work\owned\scratch", cwd, (), True)
assert mkdir.kind == filesystem_state.SUCCESS
assert r"C:\work\owned\scratch" in dict(mkdir.paths)
assert filesystem_state.resolved_location(r"\\server\share\tool", cwd, ()) is None
assert filesystem_state._mkdir_trace(r"\\server\share\tool", cwd, (), True).kind == filesystem_state.AMBIGUOUS

class NativePath:
    def __init__(self, value): self.value = value
    def is_absolute(self): return ntpath.isabs(self.value)
    def __truediv__(self, other): return NativePath(ntpath.join(self.value, other.value if isinstance(other, NativePath) else other))
    def resolve(self, strict): return self
    def __eq__(self, other): return isinstance(other, NativePath) and self.value == other.value

executable_identity.os.path = ntpath
executable_identity.Path = NativePath
lookups = []
executable_identity.shutil.which = lambda value, path=None: lookups.append(value)
assert executable_identity._path(r"C:\work\owned", cwd) == NativePath(r"C:\work\owned")
assert executable_identity._path("C:relative", cwd) is None
assert lookups == ["C:relative"]
"#,
        )
        .env("PYTHONPATH", root.join("hooks"))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()?;
    assert!(
        output.status.success(),
        "drive-anchor policy control failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn filesystem_aliases_cannot_disguise_git_mutations() -> TestResult {
    use std::os::unix::fs::symlink;

    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let git = executable("git")?;
    let linked = owned.join("git-linked");
    let copied = owned.join("git-copied");
    symlink(&git, &linked)?;
    std::fs::copy(&git, &copied)?;
    assert_case(&root, &owned, "./git-linked push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "./git-copied push --force origin topic", true, &[])?;
    assert_case(&root, &owned, "printf '%s\\n' benign", false, &[])
}

#[cfg(unix)]
#[test]
fn path_search_skips_non_executable_entries() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let fallback = workspace.path().join("fallback");
    std::fs::write(owned.join("README.md"), "not executable")?;
    std::fs::create_dir(&fallback)?;
    let path = format!("PATH={}:{}:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin", owned.display(), fallback.display());
    assert_case(&root, &owned, &format!("{path}; ln -sf /usr/bin/git {}/README.md && README.md push --force origin topic", fallback.display()), true, &[])
}

#[test]
fn sudo_chdir_forms_preserve_owned_admission_and_reject_bad_options() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &foreign, &format!("sudo -D {} git push --force origin topic", owned.display()), true, &[])?;
    assert_case(&root, &foreign, &format!("sudo --chdir={} git push --force origin topic", owned.display()), true, &[])?;
    for command in ["sudo -D", "sudo --chdir", "sudo --unknown git status"] {
        assert_case(&root, &foreign, command, true, &[])?;
    }
    assert_case(&root, &foreign, &format!("sudo -D {} git status --short", foreign.display()), false, &[])
}

pub(super) fn assert_case(root: &Path, cwd: &Path, command: &str, denied: bool, environment: &[(&str, &std::ffi::OsStr)]) -> TestResult {
    assert_event_case(root, "PreToolUse", cwd, command, denied, environment)
}

pub(super) fn assert_event_case(root: &Path, event: &str, cwd: &Path, command: &str, denied: bool, environment: &[(&str, &std::ffi::OsStr)]) -> TestResult {
    let (command, cwd) = hook_fixture_model_input(command, cwd).map_err(std::io::Error::other)?;
    let input = json!({"hook_event_name":event,"tool_name":"Bash","tool_input":{"command":command},"cwd":cwd});
    assert_input(root, input, denied, environment)
}

pub(super) fn assert_tool_case(root: &Path, tool_name: &str, tool_input: Value, denied: bool) -> TestResult {
    assert_input(
        root,
        json!({"hook_event_name":"PreToolUse","tool_name":tool_name,"tool_input":tool_input}),
        denied,
        &[],
    )
}

fn assert_input(root: &Path, input: Value, denied: bool, environment: &[(&str, &std::ffi::OsStr)]) -> TestResult {
    concern_launchers::assert_input(root, input, denied, environment)
}

#[cfg(unix)]
pub(super) fn executable(name: &str) -> TestResult<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| format!("{name} executable"))
        .map_err(Into::into)
}

pub(super) fn repository(root: &Path, name: &str, remote: &str) -> TestResult<PathBuf> {
    let path = root.join(name);
    std::fs::create_dir_all(path.join(".git"))?;
    std::fs::write(path.join(".git/config"), format!("[remote \"origin\"]\n\turl = {remote}\n"))?;
    Ok(path)
}

pub(super) fn plugin_root() -> PathBuf { codexy_runtime::paths::repository_root().join("plugins/codexy") }
