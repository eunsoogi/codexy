use super::child_lane_classification_setup_actions::{action_is_passive, setup_action_at};
use super::child_lane_classification_setup_actor::{
    SetupActor, agents_fail_closed, explicit_subject,
};
use super::child_lane_classification_setup_clause::{SENTENCE_BOUNDARY, analyze_setup_clause};
use super::child_lane_classification_setup_relative::{
    coordinates_relative_subject, main_clause_start,
};

const COMMA_BOUNDARY: &str = "__codexy_comma_boundary__";

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
                            )
                            && !coordinates_relative_subject(
                                &words,
                                predicate_start,
                                predicate_start + offset,
                                *action,
                            ))
                })
                .map(|offset| predicate_start + offset + 1)
                .unwrap_or(predicate_start);
            let end = relation_window_end(&words, *action, actions.get(position + 1).copied());
            let relative_main_start = main_clause_start(&words, start, *action);
            let window_start = relative_main_start.unwrap_or(start);
            let analysis_start = relative_main_start.unwrap_or(0);
            let window = &words[window_start..end];
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
                    negated: analyze_setup_clause(&words, analysis_start, *action, end).negated,
                    before_classification: window.iter().enumerate().any(|(index, word)| {
                        matches!(*word, "before" | "prior")
                            && !timing_phrase_is_negated(window, index)
                            && window[index + 1..]
                                .iter()
                                .take(4)
                                .any(|word| matches!(*word, "classification" | "orchestration"))
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
        COMMA_BOUNDARY | SENTENCE_BOUNDARY | "and" | "but" | "however" | "then"
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
        let mut clause_start = 0;
        for (index, character) in sentence.char_indices() {
            if clause_delimiter(sentence, index, character) {
                let clause_end = index + character.len_utf8();
                append_clause_words(&mut words, &sentence[clause_start..clause_end]);
                clause_start = clause_end;
            }
        }
        if clause_start < sentence.len() {
            append_clause_words(&mut words, &sentence[clause_start..]);
        }
        if sentence.ends_with(['.', '!', '?']) && words.last() != Some(&SENTENCE_BOUNDARY) {
            words.push(SENTENCE_BOUNDARY);
        }
    }
    words
}

fn clause_delimiter(sentence: &str, index: usize, character: char) -> bool {
    matches!(character, ',' | ';' | '(' | ')' | '[' | ']' | '—')
        || (character == '-' && !lexical_hyphen(sentence, index))
}

fn lexical_hyphen(sentence: &str, index: usize) -> bool {
    sentence[..index]
        .chars()
        .next_back()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && sentence[index + '-'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
}

fn append_clause_words<'a>(words: &mut Vec<&'a str>, clause: &'a str) {
    words.extend(
        clause
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty()),
    );
    if clause.ends_with(';') {
        words.push(SENTENCE_BOUNDARY);
    } else if clause.ends_with([',', '(', ')', '[', ']', '-', '—']) {
        words.push(COMMA_BOUNDARY);
    }
}

fn setup_action_indices<'a>(words: &'a [&'a str]) -> impl Iterator<Item = usize> + 'a {
    words
        .iter()
        .enumerate()
        .filter_map(|(index, _)| setup_action_at(words, index).map(|_| index))
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

fn actor_word(word: &str) -> Option<SetupActor> {
    match word {
        "child" => Some(SetupActor::Child),
        "parent" | "orchestrator" => Some(SetupActor::NonChild),
        _ => None,
    }
}
