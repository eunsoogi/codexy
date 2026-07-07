pub(super) fn active_child_thread_count(line: &str) -> Option<u64> {
    let (key, value) = line.split_once(':')?;
    if !has_active_child_thread_key(&key_words(key)) {
        return None;
    }
    let words = key_words(value);
    explicit_total(&words).or_else(|| fallback_count(&words))
}

pub(super) fn key_words(key: &str) -> Vec<String> {
    key.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn explicit_total(words: &[String]) -> Option<u64> {
    words
        .windows(2)
        .find_map(|window| {
            (window[0] == "total")
                .then(|| window[1].parse().ok())
                .flatten()
        })
        .or_else(|| {
            words.windows(2).find_map(|window| {
                (window[1] == "total")
                    .then(|| window[0].parse().ok())
                    .flatten()
            })
        })
}

fn fallback_count(words: &[String]) -> Option<u64> {
    let first = words.first()?;
    if first.chars().all(|c| c.is_ascii_digit()) {
        return first.parse().ok();
    }
    if let Some(count) = thread_id_entry_count(words) {
        return Some(count);
    }
    if words.iter().any(|word| word == "thread") {
        return None;
    }
    words
        .iter()
        .find(|word| word.chars().all(|character| character.is_ascii_digit()))
        .and_then(|word| word.parse().ok())
}

fn thread_id_entry_count(words: &[String]) -> Option<u64> {
    let count = words
        .windows(2)
        .filter(|window| window[0] == "thread" && window[1].chars().all(|c| c.is_ascii_digit()))
        .count();
    (count > 0).then_some(count as u64)
}

fn has_active_child_thread_key(words: &[String]) -> bool {
    words
        .iter()
        .any(|word| matches!(word.as_str(), "active" | "waiting"))
        && words.iter().any(|word| word == "child")
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "thread" | "threads"))
        && !words.iter().any(|word| word == "inactive")
        && !words
            .windows(2)
            .any(|window| window[0] == "non" && window[1] == "active")
        && (!words.iter().any(|word| {
            matches!(
                word.as_str(),
                "subagent" | "subagents" | "specialist" | "specialists"
            )
        }) || words
            .iter()
            .any(|word| matches!(word.as_str(), "exclude" | "excluding" | "excluded")))
}
