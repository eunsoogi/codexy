use super::contrast::clauses;

pub(super) fn denies_all(text: &str) -> bool {
    let clauses = clauses(text);
    let criterion = [
        progress_state(&clauses, "criterion"),
        progress_state(&clauses, "progress"),
    ]
    .into_iter()
    .max()
    .unwrap_or(Progress::Absent);
    criterion == Progress::Denied && progress_state(&clauses, "blocker") == Progress::Denied
}

fn progress_state(clauses: &[&str], subject: &str) -> Progress {
    clauses
        .iter()
        .map(|clause| state(&words(clause), subject))
        .max()
        .unwrap_or(Progress::Absent)
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Progress {
    Absent,
    Denied,
    Established,
}

fn state(words: &[String], subject: &str) -> Progress {
    let tail = words
        .iter()
        .position(|word| {
            matches!(
                word.as_str(),
                "repeat" | "repeated" | "continue" | "continues"
            )
        })
        .map(|continuation| &words[continuation + 1..])
        .unwrap_or(words);
    let Some(index) = tail.iter().position(|word| word == subject) else {
        return Progress::Absent;
    };
    let boundary = tail
        .iter()
        .position(|word| matches!(word.as_str(), "when" | "if" | "with"))
        .unwrap_or(tail.len());
    let before_subject = &tail[..index];
    let locally_denied = before_subject
        .iter()
        .rev()
        .take(4)
        .any(|word| matches!(word.as_str(), "no" | "without"));
    let continued_denial = before_subject.iter().any(|word| word == "nor")
        && before_subject
            .iter()
            .any(|word| matches!(word.as_str(), "no" | "without"));
    if locally_denied
        || continued_denial
        || (tail.first().is_some_and(|word| word == "without") && index < boundary)
    {
        return Progress::Denied;
    }
    let positive = match subject {
        "blocker" => ["removed", "removal"],
        "criterion" => ["satisfied", "newly"],
        _ => ["progress", "progress"],
    };
    tail[index..]
        .iter()
        .take(4)
        .any(|word| word == positive[0] || word == positive[1])
        .then_some(Progress::Established)
        .unwrap_or(Progress::Absent)
}

fn words(line: &str) -> Vec<String> {
    line.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}
