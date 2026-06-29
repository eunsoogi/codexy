use serde_json::Value;

pub(super) fn thread_waiting_clauses<'a>(segment: &'a str, thread: &Value) -> Vec<&'a str> {
    let review_references = review_reference_token_ranges(segment);
    thread_reference_ranges(segment, thread)
        .into_iter()
        .map(|(start, end)| {
            let clause_start = if review_references.iter().any(|&(index, _)| index < start) {
                start
            } else {
                0
            };
            let mut grouped_reference_end = end;
            let clause_end = review_references
                .iter()
                .copied()
                .find_map(|(index, reference_end)| {
                    if index <= start {
                        return None;
                    }
                    if grouped_reference_connector(&segment[grouped_reference_end..index]) {
                        grouped_reference_end = reference_end;
                        None
                    } else {
                        Some(index)
                    }
                })
                .unwrap_or(segment.len());
            &segment[clause_start..clause_end]
        })
        .collect()
}

pub(super) fn thread_referenced(text: &str, thread: &Value) -> bool {
    thread
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| has_exact_reference(text, &id.to_ascii_lowercase()))
        || comment_urls(thread).any(|url| has_exact_reference(text, &url.to_ascii_lowercase()))
}

pub(super) fn first_review_reference_start(text: &str) -> Option<usize> {
    review_reference_starts(text).into_iter().next()
}

fn thread_reference_ranges(text: &str, thread: &Value) -> Vec<(usize, usize)> {
    let mut ranges: Vec<_> = thread
        .get("id")
        .and_then(Value::as_str)
        .into_iter()
        .flat_map(|id| exact_reference_ranges(text, &id.to_ascii_lowercase()))
        .chain(
            comment_urls(thread)
                .flat_map(|url| exact_reference_ranges(text, &url.to_ascii_lowercase())),
        )
        .collect();
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn review_reference_starts(text: &str) -> Vec<usize> {
    let mut starts: Vec<_> = text
        .match_indices("prrt_")
        .map(|(index, _)| index)
        .chain(
            text.match_indices("#discussion_r")
                .map(|(index, _)| reference_token_start(text, index)),
        )
        .collect();
    starts.sort_unstable();
    starts.dedup();
    starts
}

fn review_reference_token_ranges(text: &str) -> Vec<(usize, usize)> {
    review_reference_starts(text)
        .into_iter()
        .map(|start| (start, reference_token_end(text, start)))
        .collect()
}

fn reference_token_end(text: &str, start: usize) -> usize {
    text[start..]
        .char_indices()
        .find(|&(_, character)| {
            character.is_ascii_whitespace() || matches!(character, ',' | ';' | ')' | ']' | '>')
        })
        .map_or(text.len(), |(index, _)| start + index)
}

fn reference_token_start(text: &str, index: usize) -> usize {
    text[..index]
        .rfind(|character: char| {
            character.is_ascii_whitespace() || matches!(character, '<' | '(' | '[')
        })
        .map_or(0, |index| index + 1)
}

fn grouped_reference_connector(text: &str) -> bool {
    text.split([',', '/', '&']).all(|part| {
        let part = part.trim();
        part.is_empty() || matches!(part, "and" | "or")
    })
}

fn comment_urls(thread: &Value) -> impl Iterator<Item = &str> {
    thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .filter_map(|comment| comment.get("url").and_then(Value::as_str))
}

fn has_exact_reference(text: &str, reference: &str) -> bool {
    !exact_reference_ranges(text, reference).is_empty()
}

fn exact_reference_ranges(text: &str, reference: &str) -> Vec<(usize, usize)> {
    text.match_indices(reference)
        .filter(|_| !reference.is_empty())
        .filter_map(|(start, _)| {
            let end = start + reference.len();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            (before.is_none_or(|ch| !is_reference_char(ch))
                && after.is_none_or(|ch| ch == ':' || !is_reference_char(ch)))
            .then_some((start, end))
        })
        .collect()
}

fn is_reference_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '#' | ':')
}
