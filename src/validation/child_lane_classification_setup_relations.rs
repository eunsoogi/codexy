use super::child_lane_classification_setup_actions::{action_is_passive, setup_action_at};
use super::child_lane_classification_setup_clause::{SENTENCE_BOUNDARY, analyze_setup_clause};

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

pub(super) fn has_setup_action(line: &str) -> bool {
    let relations = setup_relations(line);
    relations.iter().any(|relation| !relation.negated)
        || (relations.is_empty() && setup_action_indices(&words(line)).next().is_some())
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
                    *word == SENTENCE_BOUNDARY
                        || matches!(*word, "but" | "however")
                        || (*word == "then"
                            && words.get((predicate_start + offset).saturating_sub(1))
                                != Some(&"and"))
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
            let end = relation_window_end(&words, *action, actions.get(position + 1).copied());
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
                    negated: analyze_setup_clause(&words, 0, *action, end).negated,
                    before_classification: window.iter().enumerate().any(|(index, word)| {
                        matches!(*word, "before" | "prior")
                            && !timing_phrase_is_negated(window, index)
                            && window[index + 1..]
                                .iter()
                                .take(4)
                                .any(|word| *word == "classification")
                    }),
                })
        })
        .collect()
}

fn timing_phrase_is_negated(words: &[&str], timing: usize) -> bool {
    for index in (0..timing).rev() {
        if matches!(words[index], "not" | "never") {
            return true;
        }
        if timing_polarity_boundary(words, index) {
            return false;
        }
    }
    false
}

fn timing_polarity_boundary(words: &[&str], index: usize) -> bool {
    matches!(
        words[index],
        SENTENCE_BOUNDARY | "and" | "but" | "however" | "then"
    ) || setup_action_at(words, index).is_some()
        || actor_word(words[index]).is_some()
}

fn relation_window_end(words: &[&str], action: usize, next_action: Option<usize>) -> usize {
    let mut end = next_action.unwrap_or(words.len());
    for (offset, word) in words[action + 1..end].iter().enumerate() {
        if *word == SENTENCE_BOUNDARY
            || matches!(*word, "but" | "however")
            || (*word == "then" && words.get(action + offset) != Some(&"and"))
        {
            end = action + offset + 1;
            break;
        }
    }
    end
}

fn words(line: &str) -> Vec<&str> {
    let mut words = Vec::new();
    for sentence in line.split_inclusive(['.', '!', '?']) {
        words.extend(
            sentence
                .split(|character: char| !character.is_ascii_alphanumeric())
                .filter(|word| !word.is_empty()),
        );
        if sentence.ends_with(['.', '!', '?']) {
            words.push(SENTENCE_BOUNDARY);
        }
    }
    words
}

fn setup_action_indices<'a>(words: &'a [&'a str]) -> impl Iterator<Item = usize> + 'a {
    words
        .iter()
        .enumerate()
        .filter_map(|(index, _)| setup_action_at(words, index).map(|_| index))
}

fn explicit_subject(words: &[&str], start: usize, action: usize) -> Option<SetupActor> {
    let mut saw_non_child = false;
    let subject_start = if start > 0 && words[start - 1] == "and" {
        0
    } else {
        start
    };
    for index in subject_start..action {
        if actor_is_introduced_by(words, subject_start, index) {
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
    let actors = [start..conjunction, conjunction + 1..action]
        .map(|range| words[range].iter().find_map(|word| actor_word(word)));
    matches!(actors, [Some(left), Some(right)] if left != right)
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
