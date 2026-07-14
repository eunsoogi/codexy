use super::{
    TestResult, fixture, multiline_source, regular_lines, regular_lines_from, stderr, validate,
    write,
};

#[test]
fn touched_loc_allows_conventional_named_facade_submodules() -> TestResult {
    let repo = fixture(
        "src/validation/child_lane_thread_tool_handler_scope.rs",
        multiline_source(),
    )?;
    write(
        repo.path(),
        "src/validation/child_lane_thread_tool_handler_scope.rs",
        &("mod ownership_boundaries;\nmod lane_metadata;\nmod scope_routes;\n".to_owned()
            + &regular_lines(247)),
    )?;
    write(
        repo.path(),
        "src/validation/child_lane_thread_tool_handler_scope/ownership_boundaries.rs",
        "let summary = format!(\n    \"status\"\n);\n",
    )?;
    write(
        repo.path(),
        "src/validation/child_lane_thread_tool_handler_scope/lane_metadata.rs",
        "fn lane_metadata() {}\n",
    )?;
    write(
        repo.path(),
        "src/validation/child_lane_thread_tool_handler_scope/scope_routes.rs",
        &("fn scope_routes() {}\n".to_owned() + &regular_lines_from(247, 2)),
    )?;

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_numbered_facade_sharding() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!("mod shard_1;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/too_large/shard_1.rs",
        "let summary = format!(\n    \"status\"\n);\n",
    )?;

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_named_markdown_reference_modules() -> TestResult {
    let repo = fixture(
        "plugins/codexy/skills/wiki/references/too_large.md",
        regular_lines(252),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large.md",
        "# Reference\n\nThis reference is split into focused, navigable sections:\n\n- [Boundary](too_large/boundary.md)\n- [Index](too_large/index.md)\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/boundary.md",
        &regular_lines_from(0, 126),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/index.md",
        &regular_lines_from(126, 126),
    )?;

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_numbered_markdown_reference_fragments() -> TestResult {
    let repo = fixture(
        "plugins/codexy/skills/wiki/references/too_large.md",
        regular_lines(252),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large.md",
        "# Reference\n\n- [Part 1](too_large/part-1.md)\n- [Part 2](too_large/part-2.md)\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/part-1.md",
        &regular_lines_from(0, 126),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/part-2.md",
        &regular_lines_from(126, 126),
    )?;

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_named_markdown_rule_reference_modules() -> TestResult {
    let repo = fixture(
        "plugins/codexy/skills/wiki/references/too_large.md",
        regular_lines(252),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large.md",
        "# Reference\n\n- [C11: Canonical Placement](too_large/c11-canonical-placement.md)\n- [C18: Missing Sources](too_large/c18-missing-sources.md)\n",
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/c11-canonical-placement.md",
        &regular_lines_from(0, 126),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large/c18-missing-sources.md",
        &regular_lines_from(126, 126),
    )?;

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}
