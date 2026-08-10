use std::path::Path;

use super::semantics::destination_values;

const REPLY_LINK: &str =
    "[Plain-Language User Replies](skills/orchestration/references/plain-language-user-replies.md)";

#[test]
fn relocated_orchestration_links_have_one_platform_independent_identity() {
    let source = "MUST follow [Plain-Language User Replies](../codex-orchestration/references/plain-language-user-replies.md).";
    let destination = "MUST follow [Plain-Language User Replies](../../orchestration/references/plain-language-user-replies.md).";
    let source_value = link_value(
        source,
        Path::new("plugins/codexy/skills/debugging/SKILL.md"),
    );
    let destination_value = link_value(
        destination,
        Path::new("plugins/codexy/skills/engineering/references/diagnosis.md"),
    );

    assert_eq!(source_value, REPLY_LINK);
    assert_eq!(source_value, destination_value);
    assert!(!source_value.contains('\\'));
    assert!(!source_value.contains("skills/codex-orchestration/"));
}

#[test]
fn soft_wraps_and_link_substitutions_remain_semantically_distinct() {
    let source = "MUST follow\r\n  [Plain-Language User Replies](../codex-orchestration/references/plain-language-user-replies.md).";
    let replacement =
        "MUST follow\r\n  [Other Replies](../codex-orchestration/references/other.md).";
    let path = Path::new("plugins/codexy/skills/qa/SKILL.md");

    assert!(link_value(source, path).contains(REPLY_LINK));
    assert_ne!(link_value(source, path), link_value(replacement, path));
}

#[cfg(windows)]
#[test]
fn windows_separator_debug_and_qa_links_cannot_change_identity() {
    for source_path in [
        Path::new(r"C:\fixture\plugins\codexy\skills\debugging\SKILL.md"),
        Path::new(r"C:\fixture\plugins\codexy\skills\qa\SKILL.md"),
    ] {
        let value = link_value(
            "MUST follow [Plain-Language User Replies](../codex-orchestration/references/plain-language-user-replies.md).",
            source_path,
        );
        assert_eq!(value, REPLY_LINK);
        assert!(!value.contains('\\'));
        assert!(!value.contains("skills/codex-orchestration/"));
    }
}

fn link_value(text: &str, path: &Path) -> String {
    destination_values(text, path)
        .into_iter()
        .find(|value| value.starts_with('['))
        .expect("semantic link value")
}
