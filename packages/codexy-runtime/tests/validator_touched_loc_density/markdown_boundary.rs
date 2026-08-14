use super::*;

#[test]
fn touched_loc_ignores_markdown_fences_and_pipe_tables() -> TestResult {
    let repo = fixture("plugins/codexy/skills/example/SKILL.md", "Readable text.\n".to_owned())?;
    write(
        repo.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "~~~~sh\nfirst; second; third\n```\nstill fenced; second; third\n~~~~\nname | state | owner\n--- | --- | ---\nvalue | ready | lane\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    write(repo.path(), "plugins/codexy/skills/example/SKILL.md", "Either alpha | beta | gamma is ordinary prose; retain the context; finish the audit.\n")?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
