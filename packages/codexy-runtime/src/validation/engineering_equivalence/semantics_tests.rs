use std::path::Path;

use super::semantics::{
    destination_values, rendered_target_for_test, rendered_target_mutant, soft_wrap_mutant,
};

const REPLY: &str =
    "[Plain-Language User Replies](skills/orchestration/references/plain-language-user-replies.md)";

#[test]
fn slash_and_backslash_debug_and_qa_links_have_one_identity() {
    for path in [
        Path::new("plugins/codexy/skills/debugging/SKILL.md"),
        Path::new("plugins/codexy/skills/qa/SKILL.md"),
    ] {
        for target in [
            "../codex-orchestration/references/plain-language-user-replies.md",
            "..\\codex-orchestration\\references\\plain-language-user-replies.md",
        ] {
            let value = link(
                &format!("MUST follow [Plain-Language User Replies]({target})."),
                path,
            );
            assert_eq!(value, REPLY);
            assert!(!value.contains('\\') && !value.contains("skills/codex-orchestration/"));
        }
    }
}

#[test]
fn rendered_target_rejects_legacy_platform_separator_mutant() {
    let windows = "skills\\codex-orchestration\\references\\plain-language-user-replies.md";
    assert_eq!(
        rendered_target_for_test(windows),
        "skills/orchestration/references/plain-language-user-replies.md"
    );
    assert_ne!(
        rendered_target_mutant(windows),
        rendered_target_for_test(windows)
    );
}

#[test]
fn soft_wrapped_blocks_normalize_but_substitutions_and_mutants_differ() {
    let wrapped = "MUST follow\r\n  [Plain-Language User Replies](../codex-orchestration/references/plain-language-user-replies.md).";
    let single = "MUST follow [Plain-Language User Replies](../codex-orchestration/references/plain-language-user-replies.md).";
    let substitute =
        "MUST follow\r\n  [Other Replies](../codex-orchestration/references/other.md).";
    let path = Path::new("plugins/codexy/skills/qa/SKILL.md");
    assert_eq!(block(wrapped, path), block(single, path));
    assert_ne!(block(wrapped, path), block(substitute, path));
    assert_ne!(soft_wrap_mutant(wrapped), soft_wrap_mutant(single));
}

fn block(text: &str, path: &Path) -> String {
    destination_values(text, path)
        .into_iter()
        .next()
        .expect("block")
}
fn link(text: &str, path: &Path) -> String {
    destination_values(text, path)
        .into_iter()
        .find(|v| v.starts_with('['))
        .expect("link")
}
