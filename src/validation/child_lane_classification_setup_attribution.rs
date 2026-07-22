use super::child_lane_classification_fields::ClassificationFields;
use super::child_lane_classification_setup::line_claims_setup_before_classification;
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

#[derive(Clone, Copy, Eq, PartialEq)]
enum SetupActor {
    Child,
    NonChild,
}

pub(super) fn matched_child_branch_or_worktree_setup_clauses(line: &str) -> Vec<&str> {
    let line = trimmed_value(line);
    if line.split_once(':').is_some_and(|(key, value)| {
        ClassificationFields::records_key(metadata_key(key))
            && !line_claims_setup_before_classification(value)
    }) {
        return Vec::new();
    }
    let clauses = line
        .split_once(':')
        .filter(|(key, _)| ClassificationFields::records_key(metadata_key(key)))
        .map(|(_, value)| value)
        .unwrap_or(line);
    setup_clauses(clauses)
        .into_iter()
        .filter(|clause| !metadata_value_describes_required_negative_setup_proof(clause))
        .filter(|clause| clause_has_child_branch_or_worktree_setup(clause))
        .collect()
}

pub(super) fn clause_has_explicit_child_scope(line: &str) -> bool {
    setup_actor(line) == Some(SetupActor::Child)
        || has_child_setup_actor(line)
        || has_child_setup_subject(line)
}

fn setup_clauses(line: &str) -> Vec<&str> {
    let mut clauses = line
        .split(&[',', ';', '.'][..])
        .flat_map(|clause| clause.split(" but "))
        .flat_map(|clause| clause.split(" however "))
        .flat_map(|clause| clause.split(" and "))
        .map(str::trim)
        .collect::<Vec<_>>();
    if !clauses.iter().any(|clause| has_absent_child_setup(clause)) {
        clauses.push(line);
    }
    clauses
}

fn clause_has_child_branch_or_worktree_setup(line: &str) -> bool {
    (clause_has_explicit_child_scope(line)
        || has_codexy_branch_setup_subject(line)
        || has_unqualified_branch_or_worktree_setup(line))
        && (has_setup_action(line) || line.contains("setup"))
        && (line.contains("branch")
            || line.contains("worktree")
            || has_codexy_branch_setup_subject(line))
        && setup_actor(line) != Some(SetupActor::NonChild)
        && !has_parent_setup_subject(line)
        && !has_absent_child_setup(line)
}

fn setup_actor(line: &str) -> Option<SetupActor> {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let action = words.iter().position(|word| is_setup_action_word(word))?;
    explicit_setup_subject(&words, action).or_else(|| setup_agents_fail_closed(&words))
}

fn explicit_setup_subject(words: &[&str], action: usize) -> Option<SetupActor> {
    words[..action]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, word)| {
            (!actor_is_introduced_by(words, index))
                .then(|| actor_word(word))
                .flatten()
        })
}

fn setup_agents_fail_closed(words: &[&str]) -> Option<SetupActor> {
    let mut saw_non_child = false;
    for (index, word) in words.iter().enumerate() {
        if !actor_is_introduced_by(words, index) {
            continue;
        }
        match actor_word(word) {
            Some(SetupActor::Child) => return Some(SetupActor::Child),
            Some(SetupActor::NonChild) => saw_non_child = true,
            None => {}
        }
    }
    saw_non_child.then_some(SetupActor::NonChild)
}

fn actor_is_introduced_by(words: &[&str], actor: usize) -> bool {
    words[..actor]
        .iter()
        .rposition(|word| *word == "by")
        .is_some_and(|by| {
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

fn is_setup_action_word(word: &str) -> bool {
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
            | "set"
    )
}

fn has_unqualified_branch_or_worktree_setup(line: &str) -> bool {
    (line.contains("branch") || line.contains("worktree")) && has_setup_action(line)
}

fn has_parent_setup_subject(line: &str) -> bool {
    "parent coordination|parent implementation setup|parent setup|parent-owned setup|parent owned setup|parent-created branch|parent created branch|parent-created implementation branch|parent created implementation branch|parent-created worktree|parent created worktree|orchestrator-created branch|orchestrator created branch|orchestrator-created implementation branch|orchestrator created implementation branch|orchestrator-created worktree|orchestrator created worktree|orchestrator-created implementation worktree|orchestrator created implementation worktree"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_child_setup_actor(line: &str) -> bool {
    "child created|child-created|child thread created|child-thread created|child-thread-created|child lane created|child-lane created|child-lane-created|owning child thread created|owning child lane created|child switched|child checked out|child checkout|child thread switched|child-thread switched|child thread checked out|child-thread checked out|child thread checkout|child-thread checkout|child lane switched|child-lane switched|child lane checked out|child-lane checked out|child lane checkout|child-lane checkout|created child branch|ran git worktree add|child set up|owning child thread switched|owning child thread checked out|owning child thread checkout|owning child lane switched|owning child lane checked out|owning child lane checkout"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_child_setup_subject(line: &str) -> bool {
    "child branch|child-branch|child worktree|child-worktree|child thread branch|child-thread branch|child thread worktree|child-thread worktree|child lane branch|child-lane branch|child lane worktree|child-lane worktree|branch creation"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_codexy_branch_setup_subject(line: &str) -> bool {
    let line = line.replace(['`', '"', '\''], "");
    ((line.contains("branch codexy/") || line.contains("branch: codexy/"))
        && has_setup_action(&line))
        || ((line.contains("worktree for codexy/") || line.contains("worktree: codexy/"))
            && has_setup_action(&line))
        || (line.contains("git worktree add") && line.contains(" codexy/"))
        || (line.contains(" codexy/") && has_setup_action(&line))
}

fn has_setup_action(line: &str) -> bool {
    "created | created:| created.| created,| created;|-created | was created| got created|creation occurred|switched | switched:| switched.| switched,| switched;| was switched| checked out| checkout |git switch | setup| set up|worktree add"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_absent_child_setup(line: &str) -> bool {
    let line = trimmed_value(line);
    (setup_actor(line) == Some(SetupActor::Child)
        && (setup_action_is_negated(line) || has_negated_setup_action(line)))
        || (has_child_setup_subject(line)
            && (starts_with_absent_child_setup(line) || has_negated_setup_action(line)))
        || "no child created|no child-created|not child created|not child-created|without child created|without child-created"
            .split('|')
            .any(|marker| line.contains(marker))
}

fn setup_action_is_negated(line: &str) -> bool {
    let words = line
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| {
        is_setup_action_word(word)
            && words[index.saturating_sub(3)..index]
                .iter()
                .any(|word| matches!(*word, "no" | "not" | "never" | "without" | "neither"))
    })
}

fn starts_with_absent_child_setup(line: &str) -> bool {
    "no child branch|no child-branch|no child worktree|no child-worktree|not child branch|not child-branch|not child worktree|not child-worktree|without child branch|without child-branch|without child worktree|without child-worktree|neither child branch|neither child worktree|never child branch|never child worktree|none child branch|none child worktree"
        .split('|')
        .any(|marker| line.starts_with(marker))
}

fn has_negated_setup_action(line: &str) -> bool {
    "was not created|were not created|wasn't created|weren't created|was never created|were never created|was not set up|were not set up|wasn't set up|weren't set up"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn metadata_value_describes_required_negative_setup_proof(value: &str) -> bool {
    let value = trimmed_value(value);
    let has_negative_marker = value.contains("did not occur")
        || value.contains("didn't occur")
        || "negative test for|negative tests for|red regression for|regression test for|regression coverage for|no child branch/worktree setup occurred"
            .split('|')
            .any(|marker| value.contains(marker));
    has_negative_marker && line_claims_setup_before_classification(value)
}
