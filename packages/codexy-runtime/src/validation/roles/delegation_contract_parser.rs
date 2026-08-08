mod context;
mod tokens;

use tokens::{DELEGATION_TARGETS, is_delegation_action, words};

pub(super) use context::normalize_instruction_text;

pub(super) fn has_unnegated_permission(clause: &str) -> bool {
    let words = words(clause);
    words.iter().enumerate().any(|(index, word)| match *word {
        "may" | "can" => words
            .get(index + 1)
            .is_none_or(|next| !matches!(*next, "not" | "never")),
        "allowed" | "permitted" => {
            words.get(index.wrapping_sub(1)) != Some(&"not")
                && (words
                    .get(index + 1)
                    .is_some_and(|next| matches!(*next, "actions" | "to"))
                    || words[index + 1..].iter().any(|next| *next == "to"))
        }
        _ => false,
    })
}

pub(super) fn has_unnegated_delegation_action(
    clause: &str,
    allow_orchestration_owner: bool,
    allow_inherited_permission: bool,
) -> bool {
    let clause_words = words(clause);
    clause_words.iter().enumerate().any(|(word_index, action)| {
        if !is_delegation_action(action) {
            return false;
        }
        let prefix_words = &clause_words[..word_index];
        if !allow_inherited_permission && permission_index(prefix_words).is_none() {
            return false;
        }
        let prefix = prefix_words.join(" ");
        let action_prefix = prefix_words
            .iter()
            .rposition(|word| *word == "but")
            .map_or_else(
                || prefix.clone(),
                |but_index| prefix_words[but_index + 1..].join(" "),
            );
        let contrast_starts_with_negation = action_prefix
            .split_whitespace()
            .next()
            .is_some_and(|word| matches!(word, "not" | "never"));
        !(has_action_negation(&action_prefix)
            || (contrast_starts_with_negation && has_action_negation(&prefix)))
            && has_delegation_target(&clause_words[word_index..])
            && !(allow_orchestration_owner
                && orchestration_owner_is_actor_for_action(&clause_words, word_index))
    })
}

pub(super) fn has_unnegated_mandatory_delegation_action(
    clause: &str,
    allow_root_child_thread_creation: bool,
) -> bool {
    let clause_words = words(clause);
    clause_words.iter().enumerate().any(|(word_index, action)| {
        if !is_delegation_action(action) {
            return false;
        }
        let mandatory_prefix = mandatory_prefix(&clause_words[..word_index]);
        let creates_child_thread = allow_root_child_thread_creation
            && is_child_thread_action(action)
            && orchestration_owner_is_actor(mandatory_prefix)
            && clause_words[word_index..]
                .windows(2)
                .any(|pair| pair == ["child", "thread"]);
        has_unnegated_mandatory_permission(&mandatory_prefix.join(" "))
            && !creates_child_thread
            && has_delegation_target(&clause_words[word_index..])
    })
}

fn mandatory_prefix<'a>(prefix: &'a [&'a str]) -> &'a [&'a str] {
    prefix
        .iter()
        .rposition(|word| *word == "but")
        .map_or(prefix, |index| &prefix[index + 1..])
}

fn is_child_thread_action(action: &str) -> bool {
    matches!(
        action,
        "assign" | "assigning" | "create" | "creating" | "fork" | "forking" | "start" | "starting"
    )
}

fn orchestration_owner_is_actor(prefix: &[&str]) -> bool {
    let Some(must_index) = prefix.iter().rposition(|word| *word == "must") else {
        return false;
    };
    let subject = &prefix[..must_index];
    explicitly_names_root_orchestrator(subject)
        && !subject.iter().any(|word| DELEGATION_TARGETS.contains(word))
        && !delegates_child_creation(&prefix[must_index + 1..])
}

fn orchestration_owner_is_actor_for_action(words: &[&str], action_index: usize) -> bool {
    let Some(permission_index) = permission_index(&words[..action_index]) else {
        return false;
    };
    explicitly_names_root_orchestrator(&words[..permission_index])
        && !words[..permission_index]
            .iter()
            .any(|word| DELEGATION_TARGETS.contains(word))
        && !delegates_child_creation(&words[permission_index + 1..action_index])
}

fn explicitly_names_root_orchestrator(subject: &[&str]) -> bool {
    subject
        .windows(2)
        .any(|words| words == ["root", "orchestrator"])
}

fn permission_index(words: &[&str]) -> Option<usize> {
    words
        .iter()
        .rposition(|word| matches!(*word, "may" | "can" | "must" | "allowed" | "permitted"))
}

fn delegates_child_creation(action_prefix: &[&str]) -> bool {
    action_prefix.iter().enumerate().any(|(index, target)| {
        DELEGATION_TARGETS.contains(target)
            && action_prefix[..index]
                .iter()
                .any(|verb| is_delegation_request(verb))
    })
}

fn is_delegation_request(word: &str) -> bool {
    matches!(
        word,
        "ask"
            | "asks"
            | "asked"
            | "direct"
            | "directs"
            | "directed"
            | "instruct"
            | "instructs"
            | "instructed"
            | "request"
            | "requests"
            | "requested"
            | "tell"
            | "tells"
            | "told"
    )
}

fn has_delegation_target(words: &[&str]) -> bool {
    words
        .iter()
        .any(|candidate| DELEGATION_TARGETS.contains(candidate))
}

fn has_unnegated_mandatory_permission(prefix: &str) -> bool {
    let words = words(prefix);
    let Some(index) = words.iter().rposition(|word| *word == "must") else {
        return false;
    };
    words.get(index + 1) != Some(&"not")
        && words.get(index + 1) != Some(&"never")
        && !words[index + 1..]
            .windows(2)
            .any(|pair| pair == ["no", "circumstances"] || pair == ["any", "circumstances"])
}

fn has_action_negation(prefix: &str) -> bool {
    let words = words(prefix);
    let Some(index) = words
        .iter()
        .rposition(|word| matches!(*word, "may" | "can" | "must" | "allowed" | "permitted"))
    else {
        return false;
    };
    let following = &words[index + 1..];
    match words[index] {
        "may" | "can" => {
            following
                .iter()
                .any(|word| matches!(*word, "not" | "never"))
                || following
                    .windows(2)
                    .any(|pair| pair == ["no", "circumstances"])
        }
        "must" => {
            words.get(index + 1) == Some(&"not")
                || words.get(index + 1) == Some(&"never")
                || following
                    .windows(2)
                    .any(|pair| pair == ["no", "circumstances"])
                || following
                    .windows(2)
                    .any(|pair| pair == ["any", "circumstances"])
        }
        "allowed" | "permitted" => {
            words.get(index.wrapping_sub(1)) == Some(&"not")
                || following
                    .iter()
                    .any(|word| matches!(*word, "not" | "never"))
        }
        _ => false,
    }
}
