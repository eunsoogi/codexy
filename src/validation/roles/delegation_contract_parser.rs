const DELEGATION_TARGETS: [&str; 18] = [
    "agent",
    "agents",
    "helper",
    "helpers",
    "reviewer",
    "reviewers",
    "sentinel",
    "sentinels",
    "specialist",
    "specialists",
    "task",
    "tasks",
    "thread",
    "threads",
    "worker",
    "workers",
    "explorer",
    "explorers",
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
        let clause_words = words(clause);
        clause_words.iter().enumerate().any(|(word_index, word)| {
            if *word != action {
                return false;
            }
            let prefix_words = &clause_words[..word_index];
            let prefix = prefix_words.join(" ");
            let action_prefix = prefix_words
                .iter()
                .rposition(|word| *word == "but")
                .map_or(prefix, |but_index| prefix_words[but_index + 1..].join(" "));
            !has_action_negation(&action_prefix)
                && DELEGATION_TARGETS.iter().any(|target| {
                    clause_words[word_index..]
                        .iter()
                        .any(|candidate| candidate == target)
                })
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
        let clause_words = words(clause);
        clause_words.iter().enumerate().any(|(word_index, word)| {
            if *word != action {
                return false;
            }
            let prefix = clause_words[..word_index].join(" ");
            let prefix = prefix
                .rsplit_once(" but ")
                .map_or(prefix.as_str(), |(_, contrast)| contrast);
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
                && clause_words[word_index..]
                    .windows(2)
                    .any(|pair| pair == ["child", "thread"]);
            has_unnegated_mandatory_permission(prefix)
                && !creates_child_thread
                && DELEGATION_TARGETS.iter().any(|target| {
                    clause_words[word_index..]
                        .iter()
                        .any(|candidate| candidate == target)
                })
        })
    })
}

fn has_unnegated_mandatory_permission(prefix: &str) -> bool {
    let words = words(prefix);
    let Some(index) = words.iter().rposition(|word| *word == "must") else {
        return false;
    };
    words.get(index + 1) != Some(&"not")
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
                || following
                    .windows(2)
                    .any(|pair| pair == ["no", "circumstances"])
                || following
                    .windows(2)
                    .any(|pair| pair == ["any", "circumstances"])
        }
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
    let mut in_allowed_actions = false;
    text.lines()
        .map(str::trim)
        .map(|line| {
            if line.to_ascii_lowercase().starts_with("allowed actions")
                || line.to_ascii_lowercase().starts_with("permitted actions")
            {
                in_allowed_actions = true;
            }
            let is_item = line.starts_with("- ")
                || line.starts_with("* ")
                || line
                    .split_once('.')
                    .is_some_and(|(prefix, _)| prefix.chars().all(|c| c.is_ascii_digit()));
            let item = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .or_else(|| {
                    line.split_once(". ")
                        .filter(|(prefix, _)| {
                            prefix.chars().all(|character| character.is_ascii_digit())
                        })
                        .map(|(_, remainder)| remainder)
                });
            if in_allowed_actions && is_item {
                format!("Allowed actions: {}", item.unwrap_or(line))
            } else {
                item.unwrap_or(line).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
