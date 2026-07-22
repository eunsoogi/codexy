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

#[derive(Clone, Copy)]
enum SetupAction {
    Create,
    Switch,
    Checkout,
    Setup,
    WorktreeAdd,
}

pub(super) fn has_setup_action(line: &str) -> bool {
    let words = words(line);
    setup_action_indices(&words).next().is_some()
}

pub(super) fn setup_relations(line: &str) -> Vec<SetupRelation> {
    let words = words(line);
    let actions = setup_action_indices(&words).collect::<Vec<_>>();
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
                .enumerate()
                .rposition(|(offset, word)| {
                    matches!(*word, "then" | "but" | "however")
                        || (*word == "and"
                            && !and_coordinates_setup_subjects(
                                &words,
                                predicate_start,
                                predicate_start + offset,
                                *action,
                            ))
                })
                .map(|offset| predicate_start + offset + 1)
                .unwrap_or(predicate_start);
            let end = actions.get(position + 1).copied().unwrap_or(words.len());
            let window = &words[start..end];
            window
                .iter()
                .any(|word| matches!(*word, "branch" | "worktree"))
                .then(|| SetupRelation {
                    actor: if action_is_passive(&words, start, *action) {
                        agents_fail_closed(&words, start, end)
                            .or_else(|| explicit_subject(&words, start, *action))
                    } else {
                        explicit_subject(&words, start, *action)
                            .or_else(|| agents_fail_closed(&words, start, end))
                    },
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

fn words(line: &str) -> Vec<&str> {
    line.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn setup_action_indices<'a>(words: &'a [&'a str]) -> impl Iterator<Item = usize> + 'a {
    words
        .iter()
        .enumerate()
        .filter_map(|(index, _)| setup_action_at(words, index).map(|_| index))
}

fn explicit_subject(words: &[&str], start: usize, action: usize) -> Option<SetupActor> {
    let mut saw_non_child = false;
    for index in start..action {
        if actor_is_introduced_by(words, start, index) {
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

fn and_coordinates_setup_subjects(
    words: &[&str],
    start: usize,
    conjunction: usize,
    action: usize,
) -> bool {
    [start..conjunction, conjunction + 1..action]
        .iter()
        .all(|range| {
            words[range.clone()]
                .iter()
                .any(|word| actor_word(word).is_some())
                && words[range.clone()]
                    .iter()
                    .all(|word| actor_word(word).is_some() || is_actor_modifier(word))
        })
}

fn is_actor_modifier(word: &str) -> bool {
    matches!(
        word,
        "a" | "an" | "the" | "this" | "that" | "owning" | "thread" | "lane" | "owner" | "agent"
    )
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

fn action_is_passive(words: &[&str], start: usize, action: usize) -> bool {
    words[action.saturating_sub(3).max(start)..action]
        .iter()
        .any(|word| matches!(*word, "is" | "are" | "was" | "were" | "been" | "being"))
}

fn setup_action_at(words: &[&str], index: usize) -> Option<SetupAction> {
    match words[index] {
        "create" if has_completed_auxiliary(words, index) => Some(SetupAction::Create),
        "creates" | "created" => Some(SetupAction::Create),
        "creation" if words.get(index + 1) == Some(&"occurred") => Some(SetupAction::Create),
        "switch"
            if has_completed_auxiliary(words, index)
                || words.get(index.wrapping_sub(1)) == Some(&"git") =>
        {
            Some(SetupAction::Switch)
        }
        "switches" | "switched" => Some(SetupAction::Switch),
        "checkout" | "checkouts" => Some(SetupAction::Checkout),
        "check"
            if words.get(index + 1) == Some(&"out") && has_completed_auxiliary(words, index) =>
        {
            Some(SetupAction::Checkout)
        }
        "checked" if words.get(index + 1) == Some(&"out") => Some(SetupAction::Checkout),
        "setup" => Some(SetupAction::Setup),
        "set" | "sets" if words.get(index + 1) == Some(&"up") => Some(SetupAction::Setup),
        "add"
            if has_completed_auxiliary(words, index)
                || (index > 0 && words[index - 1] == "worktree") =>
        {
            Some(SetupAction::WorktreeAdd)
        }
        "adds" | "added" => Some(SetupAction::WorktreeAdd),
        _ => None,
    }
}

fn has_completed_auxiliary(words: &[&str], action: usize) -> bool {
    words.get(action.wrapping_sub(1)) == Some(&"did")
        || (words.get(action.wrapping_sub(1)) == Some(&"not")
            && words.get(action.wrapping_sub(2)) == Some(&"did"))
}

fn action_is_negated(words: &[&str], start: usize, action: usize) -> bool {
    words[action.saturating_sub(3).max(start)..action]
        .iter()
        .any(|word| matches!(*word, "no" | "not" | "never" | "without" | "neither"))
        || action.checked_sub(2).is_some_and(|index| {
            index >= start
                && matches!(
                    (words[index], words[index + 1]),
                    ("isn", "t") | ("aren", "t") | ("wasn", "t") | ("weren", "t")
                )
        })
}
