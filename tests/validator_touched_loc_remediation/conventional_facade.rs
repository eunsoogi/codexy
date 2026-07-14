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
    for module in ["shard_1", "shard1", "part2", "chunk3"] {
        let repo = fixture("src/too_large.rs", multiline_source())?;
        std::fs::write(
            repo.path().join("src/too_large.rs"),
            format!("mod {module};\n{}", regular_lines(249)),
        )?;
        write(
            repo.path(),
            &format!("src/too_large/{module}.rs"),
            "let summary = format!(\n    \"status\"\n);\n",
        )?;

        let output = validate(repo.path())?;
        assert!(!output.status.success(), "{module:?} unexpectedly passed");
        assert!(stderr(&output).contains("multiline collapse"));
    }
    Ok(())
}

#[test]
fn touched_loc_allows_semantic_rust_module_names_with_digits() -> TestResult {
    for module in ["http2", "s3", "ipv6"] {
        let repo = fixture("src/too_large.rs", multiline_source())?;
        write(
            repo.path(),
            "src/too_large.rs",
            &format!("mod {module};\n{}", regular_lines(249)),
        )?;
        write(
            repo.path(),
            &format!("src/too_large/{module}.rs"),
            "let summary = format!(\n    \"status\"\n);\n",
        )?;

        let output = validate(repo.path())?;
        assert!(
            output.status.success(),
            "{module:?} should be eligible\nstderr:\n{}",
            stderr(&output)
        );
    }
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
fn touched_loc_allows_semantic_markdown_reference_names_with_digits() -> TestResult {
    let repo = fixture(
        "plugins/codexy/skills/wiki/references/too_large.md",
        regular_lines(252),
    )?;
    write(
        repo.path(),
        "plugins/codexy/skills/wiki/references/too_large.md",
        "# Reference\n\n- [IPv6](too_large/ipv6.md)\n- [HTTP/2](too_large/http2.md)\n- [GPT-5 upgrade](too_large/gpt5-upgrade.md)\n",
    )?;
    for (path, start) in [("ipv6.md", 0), ("http2.md", 84), ("gpt5-upgrade.md", 168)] {
        write(
            repo.path(),
            &format!("plugins/codexy/skills/wiki/references/too_large/{path}"),
            &regular_lines_from(start, 84),
        )?;
    }

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
