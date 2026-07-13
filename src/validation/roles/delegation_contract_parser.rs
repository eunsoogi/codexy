const DELEGATION_TARGETS: [&str; 9] = [
    "agent",
    "helper",
    "reviewer",
    "sentinel",
    "specialist",
    "task",
    "thread",
    "worker",
    "explorer",
];

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

pub(super) fn has_unnegated_delegation_action(clause: &str) -> bool {
    [
        "spawn",
        "spawning",
        "delegate",
        "delegating",
        "create",
        "creating",
    ]
    .into_iter()
    .any(|action| {
        clause.match_indices(action).any(|(index, _)| {
            let prefix = &clause[..index];
            let action_prefix = prefix
                .rsplit_once(" but ")
                .map_or(prefix, |(_, contrast)| contrast);
            let suffix = &clause[index..];
            !has_action_negation(action_prefix)
                && DELEGATION_TARGETS
                    .iter()
                    .any(|target| suffix.contains(target))
        })
    })
}

pub(super) fn has_unnegated_mandatory_delegation_action(
    clause: &str,
    allow_root_child_thread_creation: bool,
) -> bool {
    [
        "spawn",
        "spawning",
        "delegate",
        "delegating",
        "create",
        "creating",
    ]
    .into_iter()
    .any(|action| {
        clause.match_indices(action).any(|(index, _)| {
            let prefix = clause[..index]
                .rsplit_once(" but ")
                .map_or(&clause[..index], |(_, contrast)| contrast);
            let suffix = &clause[index..];
            let actor_clause = prefix
                .rsplit_once(" and ")
                .map_or(prefix, |(_, current)| current);
            let root_is_actor = actor_clause
                .trim_start()
                .starts_with("the root orchestrator must")
                || actor_clause
                    .trim_start()
                    .starts_with("root orchestrator must");
            let root_is_only_actor = root_is_actor
                && !prefix.contains("sentinel")
                && !prefix.contains("helper")
                && !prefix.contains("specialist");
            let creates_child_thread = allow_root_child_thread_creation
                && action == "create"
                && root_is_only_actor
                && suffix.contains("child thread");
            has_unnegated_mandatory_permission(prefix)
                && !creates_child_thread
                && DELEGATION_TARGETS
                    .iter()
                    .any(|target| suffix.contains(target))
        })
    })
}

fn has_unnegated_mandatory_permission(prefix: &str) -> bool {
    let words = words(prefix);
    words
        .iter()
        .rposition(|word| *word == "must")
        .is_some_and(|index| words.get(index + 1) != Some(&"not"))
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
        "must" => words.get(index + 1) == Some(&"not"),
        "allowed" | "permitted" => words.get(index.wrapping_sub(1)) == Some(&"not"),
        _ => false,
    }
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect()
}

pub(super) fn normalize_instruction_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .map(|line| {
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .unwrap_or(line)
        })
        .map(|line| {
            line.split_once(". ")
                .filter(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
                .map_or(line, |(_, remainder)| remainder)
        })
        .collect::<Vec<_>>()
        .join(" ")
}
