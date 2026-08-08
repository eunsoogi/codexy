pub(super) const DELEGATION_TARGETS: [&str; 20] = [
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
    "subagent",
    "subagents",
];

const DELEGATION_ACTIONS: [&str; 12] = [
    "assign",
    "assigning",
    "spawn",
    "spawning",
    "delegate",
    "delegating",
    "create",
    "creating",
    "start",
    "starting",
    "fork",
    "forking",
];

pub(super) fn is_delegation_action(word: &str) -> bool {
    DELEGATION_ACTIONS.contains(&word)
}

pub(super) fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| {
        !character.is_ascii_alphabetic() && !matches!(character, '/' | '_')
    })
    .filter(|word| !word.is_empty())
    .collect()
}
