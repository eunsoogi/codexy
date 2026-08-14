use super::*;

#[test]
fn touched_loc_preserves_v7_lexical_boundaries() -> TestResult {
    let rust = fixture("src/lib.rs", "fn readable() {}\n".to_owned())?;
    write(rust.path(), "src/lib.rs", "fn values() { let items = [0; 3]; }\n")?;
    assert!(validate(rust.path())?.status.success());

    let javascript = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        javascript.path(),
        "src/check.js",
        "if (ready) { for (let i = 0; i < 3; i++) { break; } }\n",
    )?;
    assert!(validate(javascript.path())?.status.success());
    write(
        javascript.path(),
        "src/check.js",
        "function pattern() { return /first; second; third/; }\n",
    )?;
    assert!(validate(javascript.path())?.status.success());

    let python = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(
        python.path(),
        "scripts/check.py",
        "data = (\"#\", \"\"\"\nfirst(); second(); third();\n\"\"\")\n",
    )?;
    assert!(validate(python.path())?.status.success());
    let powershell = fixture("scripts/check.ps1", "exit 0\n".to_owned())?;
    write(
        powershell.path(),
        "scripts/check.ps1",
        "Write-Output ok # @'\nfirst; second; third\n'@\n",
    )?;
    assert!(!validate(powershell.path())?.status.success());

    let shell = fixture("scripts/release.sh", "exit 0\n".to_owned())?;
    write(
        shell.path(),
        "scripts/release.sh",
        "cat <<EOF; first; second; third\nbody\nEOF\n",
    )?;
    assert!(!validate(shell.path())?.status.success());
    write(
        shell.path(),
        "scripts/release.sh",
        "cat <<-EOF; first; second; third\nbody\nEOF\n",
    )?;
    assert!(!validate(shell.path())?.status.success());
    let workflow = fixture(".github/workflows/check.yml", "name: check\n".to_owned())?;
    write(
        workflow.path(),
        ".github/workflows/check.yml",
        "run: echo ${{ matrix.name }} && first && second && third\n",
    )?;
    assert!(!validate(workflow.path())?.status.success());
    Ok(())
}
