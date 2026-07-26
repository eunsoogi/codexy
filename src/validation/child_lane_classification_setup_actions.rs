pub(super) fn action_is_passive(words: &[&str], start: usize, action: usize) -> bool {
    words[action.saturating_sub(3).max(start)..action]
        .iter()
        .any(|word| ["is", "are", "was", "were", "been", "being", "get", "got"].contains(word))
}

pub(super) fn setup_action_at(words: &[&str], index: usize) -> Option<()> {
    match words[index] {
        "create" if has_completed_auxiliary(words, index) => Some(()),
        "creating" if action_is_passive(words, 0, index) && !is_future_plan(words, index) => {
            Some(())
        }
        "setting"
            if words.get(index + 1) == Some(&"up")
                && action_is_passive(words, 0, index)
                && !is_future_plan(words, index) =>
        {
            Some(())
        }
        "creates" | "created" if !is_future_plan(words, index) => Some(()),
        "creation" if words.get(index + 1) == Some(&"occurred") => Some(()),
        "switch"
            if has_completed_auxiliary(words, index)
                || words.get(index.wrapping_sub(1)) == Some(&"git") =>
        {
            Some(())
        }
        "switches" | "switched" => Some(()),
        "checkout" | "checkouts" => Some(()),
        "check"
            if words.get(index + 1) == Some(&"out") && has_completed_auxiliary(words, index) =>
        {
            Some(())
        }
        "checked" if words.get(index + 1) == Some(&"out") => Some(()),
        "setup" => Some(()),
        "set" | "sets" if words.get(index + 1) == Some(&"up") => Some(()),
        "add"
            if has_completed_auxiliary(words, index)
                || (index > 0 && words[index - 1] == "worktree") =>
        {
            Some(())
        }
        "adds" | "added" => Some(()),
        _ => None,
    }
}

fn has_completed_auxiliary(words: &[&str], action: usize) -> bool {
    words.get(action.wrapping_sub(1)) == Some(&"did")
        || (words.get(action.wrapping_sub(1)) == Some(&"not")
            && words.get(action.wrapping_sub(2)) == Some(&"did"))
}

fn is_future_plan(words: &[&str], action: usize) -> bool {
    words[action.saturating_sub(3)..action]
        .iter()
        .any(|word| matches!(*word, "will" | "shall"))
}
