use super::*;

#[test]
fn touched_loc_preserves_prefixes_and_javascript_spans() -> TestResult {
    let rust = fixture("src/lib.rs", "fn readable() {}\n".to_owned())?;
    write(
        rust.path(),
        "src/lib.rs",
        "fn dense(){ first(); second(); third(); /* explanation\ncontinues */ }\n",
    )?;
    assert!(!validate(rust.path())?.status.success());
    write(
        rust.path(),
        "src/lib.rs",
        "fn dense(){ first(); second(); third(); \"explanation\ncontinues\"; }\n",
    )?;
    assert!(!validate(rust.path())?.status.success());

    let javascript = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        javascript.path(),
        "src/check.js",
        "const fixture = `first(); second(); third();`;\nconst x = 1; /* first(); second(); third(); */\n",
    )?;
    assert!(validate(javascript.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_selects_shebangs_and_shell_data_boundaries() -> TestResult {
    let python = fixture("scripts/check", "#!/usr/bin/env python3\npass\n".to_owned())?;
    write(
        python.path(),
        "scripts/check",
        "#!/usr/bin/env python3\nmessage = \"first && second && third\"\n",
    )?;
    assert!(validate(python.path())?.status.success());

    let shell = fixture("scripts/release.sh", "exit 0\n".to_owned())?;
    write(
        shell.path(),
        "scripts/release.sh",
        "cat <<'EOF'\nfirst && second && third\nEOF\n",
    )?;
    assert!(validate(shell.path())?.status.success());
    write(
        shell.path(),
        "scripts/release.sh",
        "if ready; then first; second; third; fi\n",
    )?;
    assert!(!validate(shell.path())?.status.success());
    write(
        shell.path(),
        "scripts/release.sh",
        "echo notawk; awk 'BEGIN {\nfirst(); second(); third();\n}'\n",
    )?;
    assert!(validate(shell.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_uses_markdown_fence_comment_and_table_shapes() -> TestResult {
    let repo = fixture("plugins/codexy/skills/example/SKILL.md", "Readable text.\n".to_owned())?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "    ```\nIdentify the owner; retain the evidence; avoid duplicate work.\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    write(
        repo.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "~~~\nIdentify the owner; retain the evidence; avoid duplicate work.\n~~~ trailing\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    write(
        repo.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "<!-- Identify the owner; retain the evidence; avoid duplicate work. -->\nname \\| alias | state\n--- | ---\nvalue \\| primary | ready\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    write(
        repo.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "name | state | owner\n--- | ---\nIdentify the owner; retain the evidence; avoid duplicate work.\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
