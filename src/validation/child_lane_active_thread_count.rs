use super::child_lane_active_thread_count_negation::{count_word, has_active_child_thread_key};

pub(super) fn active_child_thread_count(line: &str) -> Option<u64> {
    let (key, value) = line.split_once(':')?;
    let key_words = key_words(key);
    if !has_active_child_thread_key(&key_words) {
        return None;
    }
    let words = value_words(value);
    explicit_total(&words)
        .or_else(|| labeled_component_count(&key_words, &words))
        .or_else(|| fallback_count(&words))
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
        .iter()
        .enumerate()
        .find_map(|(index, word)| {
            (word == "total")
                .then(|| count_word(words, index + 1))
                .flatten()
        })
        .or_else(|| {
            words.iter().enumerate().find_map(|(index, word)| {
                (word == "total")
                    .then(|| {
                        index
                            .checked_sub(1)
                            .and_then(|previous| count_word(words, previous))
                    })
                    .flatten()
            })
        })
}
fn fallback_count(words: &[String]) -> Option<u64> {
    if words.len() == 1 && count_word(words, 0) == Some(0) {
        return Some(0);
    }
    if let Some(first_count) = count_word(words, 0) {
        let starts_list = words.get(1).is_some_and(|word| {
            is_prefixed_thread_id(word)
                || is_non_prefixed_codex_thread_id(word)
                || (word == "thread"
                    && words
                        .get(2)
                        .is_some_and(|suffix| is_thread_id_suffix(suffix)))
        });
        return thread_id_entry_count(words)
            .filter(|count| *count > 1 && starts_list)
            .or(Some(first_count));
    }
    if let Some(count) = thread_id_entry_count(words) {
        return Some(count);
    }
    if words.iter().any(|word| word == "thread") {
        return None;
    }
    words
        .iter()
        .enumerate()
        .find_map(|(index, _)| count_word(words, index))
}
fn labeled_component_count(key_words: &[String], words: &[String]) -> Option<u64> {
    let mut counts = [None, None, None];
    let mut first_count_used = false;
    let key_has_active = key_words.iter().any(|word| word == "active");
    let key_has_waiting = key_words.iter().any(|word| word == "waiting");
    for (index, word) in words.iter().enumerate() {
        let Some(component) = component_index(word) else {
            continue;
        };
        if counts[component].is_some() {
            continue;
        }
        let Some((count, count_index)) = component_count(words, index) else {
            continue;
        };
        counts[component] = Some(count);
        first_count_used |= count_index == 0;
    }
    if counts.iter().all(Option::is_none) {
        return None;
    }
    if key_has_active && counts[0].is_none() {
        counts[0] = thread_id_entry_count(words);
    }
    if let Some(first_count) = count_word(words, 0) {
        if !first_count_used && key_has_active && counts[0].is_none() {
            counts[0] = Some(first_count);
        } else if !first_count_used && key_has_waiting && counts[1].is_none() {
            counts[1] = Some(first_count);
        }
    }
    Some(counts.iter().flatten().copied().sum())
}
fn component_count(words: &[String], index: usize) -> Option<(u64, usize)> {
    let previous_count = index
        .checked_sub(1)
        .and_then(|previous| count_word(words, previous).map(|count| (count, previous)));
    let previous_follows_component =
        previous_count.is_some_and(|(_, previous)| count_follows_component(words, previous));
    let previous_is_key_count = component_index(&words[index]) != Some(0)
        && previous_count.is_some_and(|(_, previous)| previous == 0);
    if previous_follows_component || previous_is_key_count {
        if let Some(count) = next_component_count(words, index) {
            return Some(count);
        }
    }
    previous_count
        .or_else(|| previous_component_count(words, index))
        .or_else(|| next_component_count(words, index))
}
fn count_follows_component(words: &[String], count_index: usize) -> bool {
    for previous in (0..count_index).rev() {
        if component_index(&words[previous]).is_some() {
            return true;
        }
        if count_word(words, previous).is_some() {
            return false;
        }
    }
    false
}
fn previous_component_count(words: &[String], index: usize) -> Option<(u64, usize)> {
    for previous in (0..index).rev() {
        if component_index(&words[previous]).is_some() {
            break;
        }
        if let Some(count) = count_word(words, previous) {
            return Some((count, previous));
        }
    }
    None
}
fn next_component_count(words: &[String], index: usize) -> Option<(u64, usize)> {
    for (next, word) in words.iter().enumerate().skip(index + 1) {
        if component_index(word).is_some() {
            break;
        }
        if let Some(count) = count_word(words, next) {
            return Some((count, next));
        }
    }
    None
}
fn component_index(word: &str) -> Option<usize> {
    match word {
        "active" => Some(0),
        "blocked" | "passive" | "rate-limited" | "waiting" => Some(1),
        "pending" => Some(2),
        _ => None,
    }
}
fn thread_id_entry_count(words: &[String]) -> Option<u64> {
    let mut count = 0;
    let mut index = 0;
    while index < words.len() {
        if is_prefixed_thread_id(&words[index]) {
            count += 1;
            index += 1;
        } else if words[index] == "thread"
            && words
                .get(index + 1)
                .is_some_and(|word| is_thread_id_suffix(word))
        {
            count += 1;
            index += 2;
        } else {
            if is_non_prefixed_codex_thread_id(&words[index])
                && !is_non_thread_id_context(words, index)
            {
                count += 1;
            }
            index += 1;
        }
    }
    (count > 0).then_some(count as u64)
}
fn value_words(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .replace("rate limited", "rate-limited")
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == '#')
        })
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}
fn is_prefixed_thread_id(word: &str) -> bool {
    word.strip_prefix("thread-")
        .is_some_and(|rest| !rest.is_empty())
}
fn is_thread_id_suffix(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|c| c.is_ascii_alphanumeric())
        && word.chars().any(|c| c.is_ascii_digit())
}
fn is_non_prefixed_codex_thread_id(word: &str) -> bool {
    !word.starts_with('#')
        && !word.starts_with("thread-")
        && word.len() >= 4
        && word
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
}
fn is_non_thread_id_context(words: &[String], index: usize) -> bool {
    words
        .get(index.saturating_sub(1))
        .is_some_and(|word| matches!(word.as_str(), "branch" | "worktree" | "path"))
}
