pub(super) fn has_false_blocked_or_waiting_value(value: &str) -> bool {
    if empty_heading_before_next_section(value) {
        return true;
    }
    let value = value.trim_start();
    let value = ["due to", "on", "by"]
        .iter()
        .find_map(|prefix| {
            value.strip_prefix(prefix).filter(|rest| {
                rest.chars()
                    .next()
                    .is_none_or(|ch| !ch.is_ascii_alphanumeric())
            })
        })
        .unwrap_or(value);
    let value = value
        .trim_start()
        .trim_start_matches(':')
        .trim_start()
        .trim_start_matches(['-', '*', '?'])
        .trim_start();
    let first = value
        .split(|ch: char| !matches!(ch, '/' | '0'..='9' | 'a'..='z'))
        .next()
        .unwrap_or("");
    let rest = value[first.len()..].trim_start_matches([' ', '\t']);
    let terminal = rest.chars().next().is_none_or(|ch| ".;,\n\r".contains(ch));
    let false_modifier = "active|currently|now|open|pending|remain|remaining|unresolved"
        .split('|')
        .any(|modifier| rest.starts_with(modifier));
    let false_empty = matches!(
        first,
        "0" | "zero" | "none" | "nothing" | "no" | "false" | "n/a" | "na"
    ) && (terminal || false_modifier);
    false_empty
        || matches!(first, "resolved" | "cleared")
        || value.starts_with("nothing")
        || value.starts_with("not applicable")
        || value.starts_with("no blocker")
        || value.starts_with("no waiting")
        || value.starts_with("no child")
        || value.starts_with("no related")
        || value.starts_with("no adjacent")
        || value.starts_with("no current blocker")
        || value.starts_with("no current waiting")
        || value.starts_with("no current issue")
}

fn empty_heading_before_next_section(value: &str) -> bool {
    let value = value.trim_start_matches([' ', '\t', ':']);
    let Some(first) = value.chars().find(|ch| !matches!(ch, ' ' | '\t')) else {
        return false;
    };
    if first != '\n' && first != '\r' {
        return false;
    }
    let next = value
        .lines()
        .map(|line| {
            line.trim_start()
                .trim_start_matches(['-', '*', '+'])
                .trim_start()
                .trim_matches(':')
        })
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    starts_with_pipe(
        next,
        "codex review|codex feedback|review response|review-response|review-response lane|review feedback|reviewer feedback|review thread|review comment|review comments|reviewer comments|review suggestion|review suggestions|preventive adjacent review|verification|tests|sentinel|status|follow-up|follow-ups",
    )
}

pub(super) fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(super) fn has_pipe_any(text: &str, needles: &str) -> bool {
    needles.split('|').any(|needle| text.contains(needle))
}

pub(super) fn has_current_blocker_phrase(text: &str) -> bool {
    "blocked on|blocked by|blocked due to|now blocked|goal blocked|work is blocked"
        .split('|')
        .any(|phrase| {
            text.match_indices(phrase).any(|(index, _)| {
                starts_at_word_boundary(text, index)
                    && !is_label_negated_match(&text[..index])
                    && !has_false_blocked_or_waiting_value(&text[index + phrase.len()..])
                    && !is_stale_blocker_label_value(blocker_phrase_context(text, index))
            })
        })
}

fn starts_at_word_boundary(text: &str, index: usize) -> bool {
    index == 0 || !text.as_bytes()[index - 1].is_ascii_alphanumeric()
}

fn blocker_phrase_context(text: &str, index: usize) -> &str {
    let start = text[..index].rfind(['\n', '.']).map_or(0, |i| i + 1);
    let end = text[index..]
        .find('\n')
        .map_or(text.len(), |offset| index + offset);
    &text[start..end]
}

pub(super) fn is_stale_blocker_label_value(value: &str) -> bool {
    let end = value.find('\n').unwrap_or(value.len());
    let value = value[..end].trim();
    if has_any(value, &["pending"]) {
        return has_any(value, &["previously", "historical", "earlier"])
            && has_any(value, &["resolved", "cleared"])
            && !has_current_pending_marker(value);
    }
    has_any(value, &["previous", "previously", "historical", "earlier"])
        && has_any(value, &["resolved", "cleared"])
        && !has_pipe_any(
            value,
            "now blocked|currently blocked|current blocker|current blockers|current waiting|currently pending|pending now|still blocked|still pending|still waiting",
        )
}

fn has_current_pending_marker(value: &str) -> bool {
    "now blocked|currently blocked|current blocker|current blockers|current pending|current waiting|currently pending|pending now|still blocked|still pending|still waiting"
        .split('|')
        .any(|marker| {
            value
                .match_indices(marker)
                .any(|(index, _)| !is_label_negated_match(&value[..index]))
        })
}

pub(super) fn has_unnegated_pipe(text: &str, needles: &str) -> bool {
    needles.split('|').any(|needle| has_unnegated(text, needle))
}

pub(super) fn has_unnegated_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| has_unnegated(text, needle))
}

pub(super) fn has_unnegated(text: &str, needle: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(needle) {
        let start = offset + index;
        let end = start + needle.len();
        let bounded = (start == 0 || !text.as_bytes()[start - 1].is_ascii_alphanumeric())
            && (end == text.len() || !text.as_bytes()[end].is_ascii_alphanumeric());
        if bounded
            && !is_negated_match(&text[..start], needle)
            && !is_post_negated_match(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

pub(super) fn is_label_negated_match(prefix: &str) -> bool {
    let local_start = prefix
        .rfind(['\n', ',', ';', ':', '.'])
        .map_or(0, |index| index + 1);
    prefix[local_start..].split_ascii_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
            "no" | "not" | "without" | "missing" | "lacks" | "lack" | "none"
        )
    })
}

fn is_negated_match(prefix: &str, needle: &str) -> bool {
    let sentence_start = prefix.rfind(['\n', '.']).map_or(0, |index| index + 1);
    let sentence = prefix[sentence_start..].trim_end();
    let local_start = prefix
        .rfind(['\n', ',', ';', ':', '.'])
        .map_or(0, |index| index + 1);
    let local = prefix[local_start..].trim_end();
    let historical_context =
        has_any(
            sentence,
            &["previous", "previously", "historical", "earlier"],
        ) && !has_any(local, &["now", "current", "currently", "pending", "still"]);
    let future_context = has_any(local, &["plan to", "planned to", "will ", "to run"]);
    historical_context || future_context || has_negation_word(local, needle)
}

fn has_negation_word(local: &str, needle: &str) -> bool {
    let words = local
        .split_ascii_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| {
        if *word == "not" && words.get(index + 1) == Some(&"applicable") {
            return false;
        }
        matches!(
            *word,
            "0" | "zero" | "no" | "not" | "without" | "missing" | "lacks" | "lack" | "none"
        ) || matches!(*word, "was" | "were") && needle.contains("blocked")
    })
}

fn is_post_negated_match(suffix: &str) -> bool {
    let local_end = suffix
        .find(['\n', ',', ';'])
        .or_else(|| suffix.find(". "))
        .unwrap_or(suffix.len());
    let local = suffix[..local_end]
        .trim_start_matches(|ch: char| ch.is_ascii_whitespace() || matches!(ch, ':' | '-'));
    has_false_blocked_or_waiting_value(local) && !is_not_applicable_domain_value(local)
        || has_any(local, &[" is missing", " not tested", " not covered"])
        || local.split_ascii_whitespace().any(|word| {
            matches!(
                word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
                "omit"
                    | "omits"
                    | "omitted"
                    | "skip"
                    | "skips"
                    | "skipped"
                    | "exclude"
                    | "excludes"
                    | "excluded"
                    | "lack"
                    | "lacks"
                    | "lacked"
                    | "without"
            )
        })
        || local.starts_with("s not ")
        || starts_with_pipe(
            local,
            "is not|isn't|are not|aren't|was not|wasn't|were not|weren't|do not|don't|did not|didn't|does not|doesn't|is missing|are missing|remains missing|remain missing|still missing|not added|not needed|not inspected|not run|not executed|not covered|is uncovered|are uncovered|uncovered|missing|does not exist|doesn't exist|failed|is failing|are failing|was failing|were failing|is blocked|are blocked|was blocked|were blocked|blocked|incomplete|not passing|no passing|omit|omits|omitted|skip|skips|skipped|exclude|excludes|excluded|lack|lacks|lacked|without|is planned|are planned|was planned|were planned|planned|will run|will be run|will cover|will be added|will be executed|to run|to be run|to cover|later",
        )
}

fn is_not_applicable_domain_value(local: &str) -> bool {
    local.starts_with("not applicable ") && has_any(local, &["label", "case", "branch", "state"])
}

fn starts_with_pipe(text: &str, needles: &str) -> bool {
    needles.split('|').any(|needle| text.starts_with(needle))
}
