use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn hook_validation_rejects_missing_imported_policy_modules() -> TestResult {
    for module in ["github.py", "shell_context.py"] {
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
