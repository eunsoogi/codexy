use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn hook_validation_derives_and_pins_the_runtime_import_closure() -> TestResult {
    for module in ["admission.py", "body.py", "git_command.py"] {
        let temp = tempfile::tempdir()?;
        let plugin = temp.path().join("codexy");
        copy_tree(&Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"), &plugin)?;
        std::fs::remove_file(plugin.join("hooks/codexy_policy").join(module))?;
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--plugin-root", plugin.to_str().ok_or("plugin")?, "--check-hooks"])
            .output()?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(module));
    }
    Ok(())
}

#[test]
fn hook_validation_rejects_altered_or_ambiguous_import_closure() -> TestResult {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
    for (name, replacement, expected) in [
        ("altered", "\n# changed\n", "body.py"),
        ("dynamic", "\nimport importlib\nimportlib.import_module('codexy_policy.body')\n", "dynamic"),
        ("cycle", "\nfrom .admission import evaluate\n", "cycle"),
    ] {
        let temp = tempfile::tempdir()?;
        let plugin = temp.path().join("codexy");
        copy_tree(&source, &plugin)?;
        let path = if name == "altered" { "body.py" } else if name == "dynamic" { "admission.py" } else { "body.py" };
        let target = plugin.join("hooks/codexy_policy").join(path);
        let mut contents = std::fs::read_to_string(&target)?;
        contents.push_str(replacement);
        std::fs::write(target, contents)?;
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args(["--plugin-root", plugin.to_str().ok_or("plugin")?, "--check-hooks"])
            .output()?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    Ok(())
}

#[test]
fn hook_validation_ignores_unimported_policy_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = temp.path().join("codexy");
    copy_tree(&Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"), &plugin)?;
    std::fs::write(plugin.join("hooks/codexy_policy/unused.py"), "VALUE = 1\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", plugin.to_str().ok_or("plugin")?, "--check-hooks"])
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_tree(&entry.path(), &target)?; }
        else { std::fs::copy(entry.path(), target)?; }
    }
    Ok(())
}
