pub(super) fn prohibited_goal_tools(line: &str) -> bool {
    let line = super::super::child_terminal_handoff::without_metadata_prefix(line).trim();
    if quoted(line) || inert_context(line) {
        return false;
    }
    let normalized = line
        .replace('’', "'")
        .replace("aren't", "are not")
        .replace("can't", "can not")
        .replace("mustn't", "must not")
        .replace("don't", "do not")
        .chars()
        .map(|character| match character {
            '*' | '_' | '`' | '~' => ' ',
            _ => character,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let words = normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.windows(2).enumerate().any(|(index, pair)| {
        pair[0] == "goal"
            && matches!(pair[1], "tool" | "tools" | "tooling")
            && denies_use(&words[..index], &words[index + 2..])
    })
}

fn quoted(line: &str) -> bool {
    line.chars()
        .next()
        .is_some_and(|character| matches!(character, '\"' | '\'' | '“' | '‘'))
}

fn inert_context(line: &str) -> bool {
    [
        "quoted anti-pattern:",
        "quote:",
        "example:",
        "historical ",
        "the incident wording",
        "incident wording",
        "previous wording",
        "prior wording",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn denies_use(before: &[&str], after: &[&str]) -> bool {
    ends_with(before, &["no"])
        || ends_with(before, &["never", "use"])
        || ["do", "must", "may", "shall", "can"]
            .iter()
            .any(|verb| ends_with(before, &[*verb, "not", "use"]))
        || ["authorized", "permitted", "allowed"]
            .iter()
            .any(|state| ends_with(before, &["not", *state, "to", "use"]))
        || ends_with(before, &["unauthorized", "to", "use"])
        || ["authorized", "permitted", "allowed"]
            .iter()
            .any(|state| starts_with(after, &["are", "not", *state]))
        || [
            "unauthorized",
            "prohibited",
            "disabled",
            "forbidden",
            "disallowed",
        ]
        .iter()
        .any(|state| starts_with(after, &["are", *state]))
        || ["must", "may", "shall", "can"]
            .iter()
            .any(|verb| starts_with(after, &[*verb, "not", "be", "used"]))
        || starts_with(after, &["authorization", "unauthorized"])
        || starts_with(after, &["authorization", "not", "authorized"])
}

fn ends_with(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && &words[words.len() - suffix.len()..] == suffix
}

fn starts_with(words: &[&str], prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && &words[..prefix.len()] == prefix
}
