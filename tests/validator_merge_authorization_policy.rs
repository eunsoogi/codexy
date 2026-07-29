use std::path::Path;

use crate::support;

fn fixture() -> Result<support::PluginFixture, Box<dyn std::error::Error>> {
    Ok(support::plugin_fixture_with_mutable_files(&[
        Path::new("skills/codex-orchestration/SKILL.md"),
        Path::new("skills/proof-driven-completion/SKILL.md"),
        Path::new("skills/git-workflow/references/merge-and-main-sync.md"),
    ])?)
}

#[test]
fn policy_fixture_declares_native_mutation_paths() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let declared = support::fixture_mutable_files(fixture.root()).ok_or("fixture paths")?;
    let expected = [
        "skills/codex-orchestration/SKILL.md",
        "skills/git-workflow/references/merge-and-main-sync.md",
        "skills/proof-driven-completion/SKILL.md",
    ];
    assert_eq!(declared, expected.map(Path::new).map(std::path::PathBuf::from));
    Ok(())
}

#[test]
fn policy_rejects_a_profile_that_converts_gates_to_permission()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/codex-orchestration/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nA later workflow profile can treat passing gates as permission to merge without a separate authorization record.\n");
    std::fs::write(path, text)?;

    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(
        errors
            .iter()
            .any(|error| error.contains("turn gates into merge permission")),
        "{errors:#?}"
    );
    Ok(())
}

#[test]
fn policy_rejects_profile_consent_and_a_command_decoy() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let profile = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&profile)?;
    text.push_str("\nFast profile: green gates\nimply merge consent.\n");
    std::fs::write(profile, text)?;
    let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
    let text = std::fs::read_to_string(&route)?.replace("plugins/codexy/hooks/codexy-authorized-squash-merge.sh", "gh pr merge");
    std::fs::write(route, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("before mutation")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_allows_denial_and_fenced_example() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nA workflow profile MUST NEVER imply merge authorization.\n~~~text\nFast profile: gates imply merge consent.\n~~~\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.is_empty(), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_three_line_grants_without_stitching_headings() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nFast profile:\npassing gates imply\nmerge consent.\n\n# Separate\nprofile metadata only\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_unprofiled_grants_and_allows_safe_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nPassing gates authorize merge unless the user says stop.\n1. MUST NOT grant merge permission from gates.\n2. Green gates imply merge consent.\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_later_grant_after_an_earlier_denial() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nPassing gates are not authorization, but green gates imply merge consent.\n");
    std::fs::write(path, text)?;
    assert!(!codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root()).is_empty());
    Ok(())
}

#[test]
fn policy_rejects_fallback_merge_routes_and_bullet_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
    let mut route_text = std::fs::read_to_string(&route)?.replace(
        "codexy-authorized-squash-merge.sh",
        "codexy-authorized-squash-merge.sh-decoy",
    );
    route_text.push_str("\n~~~shell\nif ! plugins/codexy/hooks/codexy-authorized-squash-merge.sh-decoy --expected-pr \"$pr_number\"; then exit 1; fi\ngh pr merge \"$pr_number\" --squash\nenv gh pr merge \"$pr_number\" --squash\n~~~\n");
    std::fs::write(route, route_text)?;
    let policy = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut policy_text = std::fs::read_to_string(&policy)?;
    policy_text.push_str("\n- Gates are not authorization.\n- Green gates imply merge consent.\n1) Passing gates authorize merge.\n");
    std::fs::write(policy, policy_text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("before mutation")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_although_grants_and_shell_sequence_bypasses() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let policy = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let policy_text = std::fs::read_to_string(&policy)?
        + "\nPassing gates are not authorization, although green gates imply merge consent.\n";
    std::fs::write(policy, policy_text)?;
    let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
    let route_text = std::fs::read_to_string(&route)? + r#"
~~~shell
plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"; gh pr merge "$pr_number" --squash
plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number" && gh pr merge "$pr_number" --squash
env FLAG=1 gh pr merge "$pr_number" --squash
FLAG=1 gh pr merge "$pr_number" --squash
~~~
"#;
    std::fs::write(route, route_text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("before mutation")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_opposite_global_rule_polarity() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let text = std::fs::read_to_string(&path)?.replace(
        "A checked contract is the sole merge authorization; generic finish, completion,\nsilence, clean gates, and a ready PR are non-authoritative signals.",
        "Generic finish is merge authorization.",
    );
    std::fs::write(path, text)?;
    assert!(!codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root()).is_empty());
    Ok(())
}

#[test]
fn policy_rejects_statement_prefix_routes_and_allows_quoted_output() -> Result<(), Box<dyn std::error::Error>> {
    for route_line in [
        "echo ready; gh pr merge \"$pr_number\" --squash",
        "printf ready && gh pr merge \"$pr_number\" --squash",
        "env -i gh pr merge \"$pr_number\" --squash",
        "env -- gh pr merge \"$pr_number\" --squash",
        "env -u TOKEN gh pr merge \"$pr_number\" --squash",
        "env --unset=TOKEN gh pr merge \"$pr_number\" --squash",
        "false || gh pr merge \"$pr_number\" --squash",
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
    ] {
        let fixture = fixture()?;
        let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
        let route_text = format!("{}\n~~~shell\n{route_line}\n~~~\n", std::fs::read_to_string(&route)?);
        std::fs::write(route, route_text)?;
        let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
        assert!(errors.iter().any(|error| error.contains("before mutation")), "unguarded route: {route_line}");
    }
    for route_line in [
        "echo 'gh pr merge \"$pr_number\" --squash'",
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
        r#"sh -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"sh -c 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lc 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lc 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lc 'gh pr view "$pr_number"'"#,
        r#"bash -lcO extglob 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lco pipefail 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash -lcO extglob 'gh pr view "$pr_number"'"#,
        r#"bash -lc -O extglob 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -lc -o pipefail 'echo "gh pr merge $pr_number --squash"'"#,
        r#"bash --noprofile -c 'plugins/codexy/hooks/codexy-authorized-squash-merge.sh --expected-pr "$pr_number"'"#,
        r#"bash -q -c 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -oc 'gh pr merge "$pr_number" --squash'"#,
        "bash -lcO extglob",
        "bash -lc -O",
        r#"bash -lc -q 'gh pr merge "$pr_number" --squash'"#,
        r#"bash -- -c 'gh pr merge "$pr_number" --squash'"#,
        "{gh pr merge \"$pr_number\" --squash;}",
        "if true; then ! gh pr view \"$pr_number\"; fi",
    ] {
        let fixture = fixture()?;
        let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
        let route_text = format!("{}\n~~~shell\n{route_line}\n~~~\n", std::fs::read_to_string(&route)?);
        std::fs::write(route, route_text)?;
        assert!(codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root()).is_empty());
    }
    Ok(())
}

#[test]
fn policy_rejects_coexisting_opposite_polarity_and_uppercase_although() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let global = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let global_text = std::fs::read_to_string(&global)? + "\nGeneric finish is merge authorization.\nPassing gates are not authorization, ALTHOUGH green gates imply merge consent.\n";
    std::fs::write(global, global_text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("global merge-authorization prohibition")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}
