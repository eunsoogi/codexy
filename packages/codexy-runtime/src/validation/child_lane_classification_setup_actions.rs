use super::child_lane_classification_setup_clause::{SENTENCE_BOUNDARY, analyze_setup_clause};

pub(super) fn action_is_passive(words: &[&str], start: usize, action: usize) -> bool {
    words[action.saturating_sub(3).max(start)..action]
        .iter()
        .any(|word| ["is", "are", "was", "were", "been", "being", "get", "got"].contains(word))
}

pub(super) fn setup_action_at(words: &[&str], index: usize) -> Option<()> {
    match words[index] {
        "create" if has_direct_auxiliary(words, index) => Some(()),
        "creating"
            if action_is_passive(words, 0, index)
                && !analyze_setup_clause(words, 0, index, words.len()).prospective =>
        {
            Some(())
        }
        "setting"
            if words.get(index + 1) == Some(&"up")
                && is_governing_progressive_setup(words, index)
                && !analyze_setup_clause(words, 0, index, words.len()).prospective =>
        {
            Some(())
        }
        "creates" | "created"
            if !analyze_setup_clause(words, 0, index, words.len()).prospective =>
        {
            Some(())
        }
        "creation" if words.get(index + 1) == Some(&"occurred") => Some(()),
        "switch"
            if has_direct_auxiliary(words, index)
                || words.get(index.wrapping_sub(1)) == Some(&"git") =>
        {
            Some(())
        }
        "switches" | "switched" => Some(()),
        "checkout" | "checkouts" => Some(()),
        "check" if words.get(index + 1) == Some(&"out") && has_direct_auxiliary(words, index) => {
            Some(())
        }
        "checked" if words.get(index + 1) == Some(&"out") => Some(()),
        "setup" => Some(()),
        "set" | "sets" if words.get(index + 1) == Some(&"up") => Some(()),
        "add"
            if has_direct_auxiliary(words, index)
                || (index > 0 && words[index - 1] == "worktree") =>
        {
            Some(())
        }
        "adds" | "added" => Some(()),
        _ => None,
    }
}

fn is_governing_progressive_setup(words: &[&str], action: usize) -> bool {
    let analysis = analyze_setup_clause(words, 0, action, words.len());
    let Some(auxiliary) = words[analysis.start..action]
        .iter()
        .rposition(is_progressive_auxiliary)
        .map(|offset| analysis.start + offset)
        .or_else(|| shared_progressive_auxiliary(words, analysis.start))
    else {
        return false;
    };
    let predicate = &words[analysis.start.max(auxiliary + 1)..action];
    !predicate.iter().enumerate().any(|(index, word)| {
        word.ends_with("ing") && !predicate[index + 1..].iter().any(is_predicate_coordinator)
    })
}

fn shared_progressive_auxiliary(words: &[&str], clause_start: usize) -> Option<usize> {
    let connector = *words.get(clause_start.checked_sub(1)?)?;
    let sentence_start = words[..clause_start]
        .iter()
        .rposition(|word| *word == SENTENCE_BOUNDARY)
        .map(|boundary| boundary + 1)
        .unwrap_or(0);
    (connector != SENTENCE_BOUNDARY).then(|| {
        words[sentence_start..clause_start - 1]
            .iter()
            .rposition(is_progressive_auxiliary)
            .map(|offset| sentence_start + offset)
    })?
}

fn is_predicate_coordinator(word: &&str) -> bool {
    matches!(*word, "and" | "but")
}

fn is_progressive_auxiliary(word: &&str) -> bool {
    matches!(
        *word,
        "is" | "are" | "was" | "were" | "been" | "being" | "get" | "got"
    )
}

fn has_direct_auxiliary(words: &[&str], action: usize) -> bool {
    let auxiliary = words.get(action.wrapping_sub(1));
    matches!(auxiliary, Some(&"do" | &"does" | &"did"))
        || (auxiliary == Some(&"not")
            && matches!(
                words.get(action.wrapping_sub(2)),
                Some(&"do" | &"does" | &"did")
            ))
}
