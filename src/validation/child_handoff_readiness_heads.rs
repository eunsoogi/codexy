pub(super) fn claimed_pushed_heads(text: &str) -> Vec<String> {
    text.split(|ch| matches!(ch, '\n' | ';'))
        .flat_map(|line| line.split(". "))
        .filter(|sentence| {
            let lower = sentence.to_ascii_lowercase();
            lower.contains("pushed")
                || lower.contains("synced")
                || lower.contains("local head")
                || lower.contains("remote/pr head match")
        })
        .flat_map(head_refs_after_markers)
        .collect()
}

fn head_refs_after_markers(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let (mut before_previous, mut previous) = (String::new(), String::new());
    for token in text.split_whitespace() {
        let candidate = token.trim_matches(|ch: char| !ch.is_ascii_hexdigit());
        let follows_named_marker = matches!(
            previous.as_str(),
            "at" | "head" | "head:" | "sha" | "sha:" | "commit" | "commit:"
        );
        let follows_compact_pushed_marker = matches!(previous.as_str(), "pushed" | "pushed:")
            || (matches!(previous.as_str(), "yes" | "yes:")
                && matches!(before_previous.as_str(), "pushed" | "pushed:"));
        let follows_head_match_marker = matches!(previous.as_str(), "yes" | "yes:")
            && matches!(before_previous.as_str(), "match" | "match:");
        let is_pr_head = matches!(before_previous.as_str(), "pr" | "request")
            && matches!(previous.as_str(), "head" | "head:");
        if (follows_named_marker || follows_compact_pushed_marker || follows_head_match_marker)
            && !is_pr_head
            && (7..=40).contains(&candidate.len())
            && candidate.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            refs.push(candidate.to_ascii_lowercase());
        }
        before_previous = previous;
        previous = token
            .trim_matches(|ch: char| !ch.is_ascii_alphabetic() && ch != ':')
            .to_ascii_lowercase();
    }
    refs
}
