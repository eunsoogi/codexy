#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SetupActor {
    Child,
    NonChild,
}

#[derive(Clone, Copy)]
pub(super) struct SetupRelation {
    pub(super) actor: Option<SetupActor>,
    pub(super) negated: bool,
    pub(super) before_classification: bool,
}

pub(super) fn setup_relations(line: &str) -> Vec<SetupRelation> {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let actions = words
        .iter()
        .enumerate()
        .filter_map(|(index, word)| {
            is_setup_action_word(word, words.get(index + 1)).then_some(index)
        })
        .collect::<Vec<_>>();
    actions
        .iter()
        .enumerate()
        .filter_map(|(position, action)| {
            let predicate_start = position
                .checked_sub(1)
                .map(|previous| actions[previous] + 1)
                .unwrap_or(0);
            let start = words[predicate_start..*action]
                .iter()
                .rposition(|word| matches!(*word, "then" | "but" | "however" | "and"))
                .map(|offset| predicate_start + offset + 1)
                .unwrap_or(predicate_start);
            let end = actions.get(position + 1).copied().unwrap_or(words.len());
            let window = &words[start..end];
            window
                .iter()
                .any(|word| matches!(*word, "branch" | "worktree"))
                .then(|| SetupRelation {
                    actor: explicit_subject(&words, start, *action)
                        .or_else(|| agents_fail_closed(&words, start, end)),
                    negated: action_is_negated(&words, start, *action),
                    before_classification: window.iter().enumerate().any(|(index, word)| {
                        *word == "before"
                            && window[index + 1..]
                                .iter()
                                .take(4)
                                .any(|word| *word == "classification")
                    }),
                })
        })
        .collect()
}

fn explicit_subject(words: &[&str], start: usize, action: usize) -> Option<SetupActor> {
    words[start..action]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(offset, word)| {
            let index = start + offset;
            (!actor_is_introduced_by(words, start, index))
                .then(|| actor_word(word))
                .flatten()
        })
}

fn agents_fail_closed(words: &[&str], start: usize, end: usize) -> Option<SetupActor> {
    let mut saw_non_child = false;
    for index in start..end {
        if !actor_is_introduced_by(words, start, index) {
            continue;
        }
        match actor_word(words[index]) {
            Some(SetupActor::Child) => return Some(SetupActor::Child),
            Some(SetupActor::NonChild) => saw_non_child = true,
            None => {}
        }
    }
    saw_non_child.then_some(SetupActor::NonChild)
}

fn actor_is_introduced_by(words: &[&str], start: usize, actor: usize) -> bool {
    words[start..actor]
        .iter()
        .rposition(|word| *word == "by")
        .is_some_and(|offset| {
            let by = start + offset;
            words[by + 1..actor].iter().all(|word| {
                matches!(
                    *word,
                    "a" | "an" | "the" | "this" | "that" | "its" | "our" | "owning"
                )
            })
        })
}

fn actor_word(word: &str) -> Option<SetupActor> {
    match word {
        "child" => Some(SetupActor::Child),
        "parent" | "orchestrator" => Some(SetupActor::NonChild),
        _ => None,
    }
}

fn is_setup_action_word(word: &str, next: Option<&&str>) -> bool {
    matches!(
        word,
        "create"
            | "created"
            | "creation"
            | "switch"
            | "switched"
            | "checkout"
            | "checked"
            | "setup"
    ) || (word == "set" && next == Some(&&"up"))
}

fn action_is_negated(words: &[&str], start: usize, action: usize) -> bool {
    words[action.saturating_sub(3).max(start)..action]
        .iter()
        .any(|word| matches!(*word, "no" | "not" | "never" | "without" | "neither"))
}
