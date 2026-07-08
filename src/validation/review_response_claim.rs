pub(super) fn claims_review_response(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    has_any(
        &text,
        &[
            "accepted no-change rationale",
            "accepted no change rationale",
            "no-change rationale documented",
            "no change rationale documented",
        ],
    ) || review_feedback_segments(&text).any(|segment| {
        "addressed addresses addressing applied fixed fixes handled implemented responded resolved resolve resolves updated"
            .split_whitespace()
            .any(|phrase| has_unnegated_action(segment, phrase))
    })
}

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn review_feedback_segments(text: &str) -> impl Iterator<Item = &str> {
    let mut section = false;
    text.split_inclusive(['.', '\n', ';'])
        .filter(move |segment| {
            let has_context =
                "codex review|codex feedback|review response|review-response|review feedback|reviewer feedback|review thread|review comment|review comments|reviewer comments|review suggestion|review suggestions"
                    .split('|')
                    .any(|term| segment.contains(term));
            let trimmed = segment.trim_start();
            let no_feedback = segment.contains(": none")
                || "none from codex|no review feedback|no feedback|no comment|no comments|no suggestion|no suggestions"
                    .split('|')
                    .any(|term| segment.contains(term));
            let output = "comment|feedback|suggestion|thread"
                .split('|')
                .any(|t| segment.contains(t));
            let matches = has_context || (section && !trimmed.is_empty());
            section = !no_feedback
                && ((has_context
                    && (output || segment.trim_end().ends_with(':') || trimmed.starts_with('#')))
                    || segment.contains("review response:")
                    || segment.contains("review-response:")
                    || (section && (trimmed.starts_with('-') || trimmed.is_empty())));
            matches
        })
}

fn has_unnegated_action(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if !is_word_match(text, start, end) {
            offset = end;
            rest = &text[offset..];
            continue;
        }
        let prefix = &text[..start];
        let local_prefix = local_action_prefix(prefix);
        let clean_review_prefix = prefix.trim_end().ends_with("codex review passed,")
            && !text[start..].contains("review");
        if !clean_review_prefix
            && !"review response: none|review-response: none|review feedback: none|reviewer feedback: none|review thread: none|review comments: none|reviewer comments: none|none from codex"
                .split('|')
                .any(|term| prefix.contains(term))
            && !"no review feedback was |no review feedback |no feedback was |no feedback |not "
                .split('|')
                .any(|negation| local_prefix.contains(negation))
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn is_word_match(text: &str, start: usize, end: usize) -> bool {
    let b = text.as_bytes();
    !b.get(start.wrapping_sub(1))
        .is_some_and(u8::is_ascii_alphanumeric)
        && !b.get(end).is_some_and(u8::is_ascii_alphanumeric)
}

fn local_action_prefix(prefix: &str) -> &str {
    let p = prefix.rfind(['\n', ',', ';']).map(|index| index + 1);
    let s = prefix.rfind(". ").map(|index| index + 1);
    let c = prefix.rfind(" but ").map(|index| index + 5);
    let start = [p, s, c].into_iter().flatten().max().unwrap_or(0);
    &prefix[start..]
}
