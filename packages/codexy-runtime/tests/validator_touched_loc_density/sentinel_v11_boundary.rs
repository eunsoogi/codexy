use super::*;

#[test]
fn touched_loc_exposes_nested_javascript_template_interpolation() -> TestResult {
    let repo = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        repo.path(),
        "src/check.js",
        "const label = `${`${(() => { first(); second(); third(); })()}`}`;\n",
    )?;
    assert!(!validate(repo.path())?.status.success());

    let multiline = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        multiline.path(),
        "src/check.js",
        "const label = `${`\n${(() => { first(); second(); third(); })()}\n`}`;\n",
    )?;
    assert!(!validate(multiline.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_distinguishes_shell_heredocs_from_comments_and_shifts() -> TestResult {
    let comment = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        comment.path(),
        "scripts/check.sh",
        "# cat <<EOF\nfirst && second && third\nEOF\n",
    )?;
    assert!(!validate(comment.path())?.status.success());

    let shift = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        shift.path(),
        "scripts/check.sh",
        "value=$((1 << 2))\nfirst && second && third\n",
    )?;
    assert!(!validate(shift.path())?.status.success());

    let here_string = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        here_string.path(),
        "scripts/check.sh",
        "value=<<<EOF\nfirst && second && third\n",
    )?;
    assert!(!validate(here_string.path())?.status.success());

    let quoted = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        quoted.path(),
        "scripts/check.sh",
        "echo \"escaped quote: \\\" <<EOF; still quoted\"\nfirst && second && third\n",
    )?;
    assert!(!validate(quoted.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_masks_quoted_shell_heredoc_words() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "cat <<-'E'\"OF\"\nfirst && second && third\n\tEOF\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}
