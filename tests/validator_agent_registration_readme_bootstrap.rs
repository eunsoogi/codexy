use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[cfg(unix)]
#[test]
fn readme_resolver_accepts_only_one_canonical_official_install() -> TestResult {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let temp = tempfile::tempdir()?;
    let temp_root = temp.path().canonicalize()?;
    let valid_root = make_plugin(&temp_root.join("plugin with spaces;$()\nline"))?;
    let script = readme_resolver()?;

    let valid = run_resolver(&script, &plugin_list(&valid_root, true, true))?;
    assert!(valid.status.success(), "stderr:\n{}", stderr(&valid));
    assert_eq!(stdout(&valid), "bootstrap-ran\n");

    let cases = [
        ("disabled", plugin_list(&valid_root, true, false)),
        ("not-installed", plugin_list(&valid_root, false, true)),
        (
            "wrong-origin",
            plugin_list_with_origin(&valid_root, "https://example.invalid/codexy.git"),
        ),
        (
            "relative-path",
            plugin_list(Path::new("relative/plugin"), true, true),
        ),
        (
            "duplicate",
            serde_json::json!({"installed": [
                plugin_entry(&valid_root, true, true, "https://github.com/eunsoogi/codexy.git"),
                plugin_entry(&valid_root, true, true, "https://github.com/eunsoogi/codexy.git")
            ]}),
        ),
    ];
    for (label, payload) in cases {
        let output = run_resolver(&script, &payload)?;
        assert!(!output.status.success(), "{label} resolver input succeeded");
        assert!(stdout(&output).is_empty(), "{label} executed bootstrap");
    }

    let linked_root = temp_root.join("linked-plugin");
    symlink(&valid_root, &linked_root)?;
    let parent_symlink = run_resolver(&script, &plugin_list(&linked_root, true, true))?;
    assert!(!parent_symlink.status.success());
    assert!(stdout(&parent_symlink).is_empty());

    let final_link_root = make_plugin(&temp_root.join("final-link"))?;
    let bootstrap = final_link_root.join("bootstrap-codexy-agents");
    std::fs::remove_file(&bootstrap)?;
    let target = temp_root.join("outside-bootstrap");
    std::fs::write(&target, "#!/bin/sh\nprintf 'should-not-run\\n'\n")?;
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
    symlink(&target, &bootstrap)?;
    let final_symlink = run_resolver(&script, &plugin_list(&final_link_root, true, true))?;
    assert!(!final_symlink.status.success());
    assert!(stdout(&final_symlink).is_empty());

    let malformed = run_raw(&script, b"{not-json")?;
    assert!(!malformed.status.success());
    assert!(stdout(&malformed).is_empty());
    Ok(())
}

#[cfg(unix)]
fn make_plugin(root: &Path) -> TestResult<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(root.join(".codex-plugin"))?;
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        r#"{"name":"codexy","repository":"https://github.com/eunsoogi/codexy"}"#,
    )?;
    let bootstrap = root.join("bootstrap-codexy-agents");
    std::fs::write(&bootstrap, "#!/bin/sh\nprintf 'bootstrap-ran\\n'\n")?;
    std::fs::set_permissions(&bootstrap, std::fs::Permissions::from_mode(0o755))?;
    Ok(root.to_path_buf())
}

fn readme_resolver() -> TestResult<String> {
    let readme = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))?;
    let line = readme
        .lines()
        .find(|line| {
            line.starts_with("codex plugin list --marketplace codexy --json | python3 -c '")
        })
        .ok_or("README bootstrap command missing")?;
    let (_, script) = line
        .split_once("python3 -c '")
        .ok_or("Python resolver missing")?;
    Ok(script
        .strip_suffix('\'')
        .ok_or("resolver quote missing")?
        .to_string())
}

fn plugin_list(root: &Path, installed: bool, enabled: bool) -> serde_json::Value {
    plugin_list_with_entry(plugin_entry(
        root,
        installed,
        enabled,
        "https://github.com/eunsoogi/codexy.git",
    ))
}

fn plugin_list_with_origin(root: &Path, origin: &str) -> serde_json::Value {
    plugin_list_with_entry(plugin_entry(root, true, true, origin))
}

fn plugin_list_with_entry(entry: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"installed": [entry]})
}

fn plugin_entry(root: &Path, installed: bool, enabled: bool, origin: &str) -> serde_json::Value {
    serde_json::json!({
        "pluginId": "codexy@codexy",
        "name": "codexy",
        "marketplaceName": "codexy",
        "installed": installed,
        "enabled": enabled,
        "source": {"source": "local", "path": root},
        "marketplaceSource": {"sourceType": "git", "source": origin}
    })
}

fn run_resolver(script: &str, value: &serde_json::Value) -> TestResult<std::process::Output> {
    run_raw(script, &serde_json::to_vec(value)?)
}

fn run_raw(script: &str, input: &[u8]) -> TestResult<std::process::Output> {
    let mut child = Command::new("python3")
        .args(["-c", script])
        .env("PYTHONOPTIMIZE", "2")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("resolver stdin missing")?
        .write_all(input)?;
    Ok(child.wait_with_output()?)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
