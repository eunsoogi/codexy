fn route_errors(route_line: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let fixture = super::fixture()?;
    let route = fixture
        .root()
        .join("skills/git-workflow/references/merge-and-main-sync.md");
    let route_text = format!("{}\n~~~shell\n{route_line}\n~~~\n", std::fs::read_to_string(&route)?);
    std::fs::write(route, route_text)?;
    Ok(codexy_runtime::validation::merge_authorization_policy_diagnostics(
        fixture.root(),
    ))
}

#[test]
fn policy_rejects_statement_prefix_routes() -> Result<(), Box<dyn std::error::Error>> {
    for route_line in [
        "echo ready; gh pr merge \"$pr_number\" --squash",
        "printf ready && gh pr merge \"$pr_number\" --squash",
        "env -u TOKEN gh pr merge \"$pr_number\" --squash",
        "env --unset=TOKEN gh pr merge \"$pr_number\" --squash",
        "false || gh pr merge \"$pr_number\" --squash",
        "true | gh pr merge \"$pr_number\" --squash",
        "gh pr merge \"$pr_number\" --squash | cat",
        "gh pr \\\nmerge \"$pr_number\" --squash",
        "env -i gh pr merge \"$pr_number\" --squash",
        "env -- gh pr merge \"$pr_number\" --squash",
        "if gh pr merge \"$pr_number\" --squash; then exit 1; fi",
        "command gh pr merge \"$pr_number\" --squash",
        "! gh pr merge \"$pr_number\" --squash",
        "if true; then gh pr merge \"$pr_number\" --squash; fi",
        "if true; then ! gh pr merge \"$pr_number\" --squash; fi",
        "while true; do gh pr merge \"$pr_number\" --squash; done",
        "while true; do { gh pr merge \"$pr_number\" --squash; }; done",
        r#"sh -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lc 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lcO extglob 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lco pipefail 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lc -O extglob 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lc -o pipefail 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lc -- 'gh pr merge "$pr_number" --squash'"#,
        r#"bash --noprofile -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash --rcfile /dev/null -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -lcO extglob"#,
        r#"bash -lc -O"#,
        r#"bash -lc -q 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -q -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -oc 'gh pr merge "$pr_number" --squash'"#,
    ] {
        let errors = route_errors(route_line)?;
        assert!(
            errors.iter().any(|error| error.contains("before mutation")),
            "unguarded route: {route_line} ({errors:#?})"
        );
    }
    Ok(())
}

#[test]
fn policy_allows_inert_and_authorized_logical_shell_routes()
-> Result<(), Box<dyn std::error::Error>> {
    for route_line in [
        "echo 'gh pr merge \"$pr_number\" --squash' | cat",
        "printf 'gh pr \\\nmerge \"$pr_number\" --squash' | cat",
        "env FLAG=1 plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "env -i plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "env -- plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "command plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "if plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"; then exit 0; fi",
        "! plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "if true; then plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"; fi",
        "if true; then ! plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"; fi",
        "while true; do plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"; done",
        "while true; do { plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"; }; done",
        "true | plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\"",
        "plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \"$pr_number\" | cat",
        "plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr \\\n\"$pr_number\"",
        r#"sh -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"sh -c 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lc 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lc 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lc 'gh pr view "$pr_number"'"#,
        r#"bash -lcO extglob 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lco pipefail 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lcO extglob 'gh pr view "$pr_number"'"#,
        r#"bash -lc -O extglob 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lc -o pipefail 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash --noprofile -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -- -c 'gh pr merge "$pr_number" --squash'"#,
        "{gh pr merge \"$pr_number\" --squash;}",
        "if true; then ! gh pr view \"$pr_number\"; fi",
    ] {
        let errors = route_errors(route_line)?;
        assert!(errors.is_empty(), "{route_line}: {errors:#?}");
    }
    Ok(())
}
