use std::path::Path;

use crate::support;

fn fixture() -> Result<support::PluginFixture, Box<dyn std::error::Error>> {
    Ok(support::plugin_fixture_with_mutable_files(&[Path::new(
        "skills/git-workflow/references/merge-and-main-sync.md",
    )])?)
}

fn policy_errors(route_line: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
    let route_text = format!("{}\n~~~shell\n{route_line}\n~~~\n", std::fs::read_to_string(&route)?);
    std::fs::write(route, route_text)?;
    Ok(codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root()))
}

#[test]
fn policy_rejects_generic_shell_long_option_merge_routes() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        r#"bash --verbose -c 'gh pr merge "$pr_number" --squash'"#,
        r#"zsh --xtrace -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash --- -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash --rcfile -c 'gh pr merge "$pr_number" --squash'"#,
    ] {
        assert!(policy_errors(route)?.iter().any(|error| error.contains("before mutation")), "{route}");
    }
    Ok(())
}

#[test]
fn policy_allows_generic_long_option_wrapper_and_delimiter_controls() -> Result<(), Box<dyn std::error::Error>> {
    for route in [
        r#"bash --verbose -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"zsh --xtrace -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash --rcfile /dev/null -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -- -c 'gh pr merge "$pr_number" --squash'"#,
    ] {
        assert!(policy_errors(route)?.is_empty(), "{route}");
    }
    Ok(())
}
