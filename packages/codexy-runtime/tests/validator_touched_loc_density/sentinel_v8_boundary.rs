use super::*;

#[test]
fn touched_loc_detects_javascript_before_and_inside_loop_boundaries() -> TestResult {
    let repo = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        repo.path(),
        "src/check.js",
        "if (ready) { first(); second(); third(); for (;;) { break; } }\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    write(
        repo.path(),
        "src/check.js",
        "items.for(function () { first(); second(); third(); });\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_tracks_javascript_regex_state_boundaries() -> TestResult {
    let repo = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        repo.path(),
        "src/check.js",
        "const pattern = () => /first; second; third/;\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    write(
        repo.path(),
        "src/check.js",
        "const pattern = /[/;]/; first(); second(); third();\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_tracks_python_and_powershell_after_closed_strings() -> TestResult {
    let python = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(
        python.path(),
        "scripts/check.py",
        "data = '\"\"\"' or \"\"\"\nfirst(); second(); third();\n\"\"\"\n",
    )?;
    assert!(validate(python.path())?.status.success());

    let powershell = fixture("scripts/check.ps1", "exit 0\n".to_owned())?;
    write(
        powershell.path(),
        "scripts/check.ps1",
        "Write-Output \"ok\"; $data = @'\nfirst; second; third\n'@\n",
    )?;
    assert!(validate(powershell.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_preserves_shell_heredoc_redirection_suffixes() -> TestResult {
    let repo = fixture("scripts/release.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/release.sh",
        "cat <<EOF>out\nbody\nEOF\nfirst && second && third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
