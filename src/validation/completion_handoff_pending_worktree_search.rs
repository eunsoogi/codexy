use super::completion_handoff_pending_worktree_text::{
    char_window_start, has_any, has_nearby_negation, phrase_has_boundaries,
};

pub(super) fn has_search_dimension(text: &str, dimensions: &str) -> bool {
    for marker in ["list_threads searches by", "searches by", "searched by"] {
        let mut rest = text;
        let mut offset = 0;
        while let Some(index) = rest.find(marker) {
            let start = offset + index;
            let clause_start = start + marker.len();
            if phrase_has_boundaries(text, start, clause_start)
                && !has_nearby_negation(&text[char_window_start(text, start, 16)..start])
            {
                let clause_end = text[clause_start..]
                    .find(['.', ';', '\n'])
                    .map_or(text.len(), |end| clause_start + end);
                let clause = &text[clause_start..clause_end];
                if has_any(clause, dimensions) && !has_negated_dimension(clause, dimensions) {
                    return true;
                }
            }
            offset = clause_start;
            rest = &text[offset..];
        }
    }
    false
}

fn has_negated_dimension(clause: &str, dimensions: &str) -> bool {
    dimensions.split('|').any(|dimension| {
        has_any(
            clause,
            &format!("without searching {dimension}|without {dimension}|not searching {dimension}"),
        )
    })
}
