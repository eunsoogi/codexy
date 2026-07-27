use super::child_lane_classification_setup_clause::SENTENCE_BOUNDARY;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SetupActor {
    Child,
    NonChild,
}

pub(super) fn explicit_subject(words: &[&str], start: usize, action: usize) -> Option<SetupActor> {
    select_subject(words, start, action).or_else(|| {
        let inherited_start = words[..start]
            .iter()
            .rposition(|word| *word == SENTENCE_BOUNDARY || matches!(*word, "but" | "however"))
            .map(|index| index + 1)
            .unwrap_or(0);
        select_subject(words, inherited_start, action)
    })
}

fn select_subject(words: &[&str], start: usize, end: usize) -> Option<SetupActor> {
    together_with_subject_actor(words, start, end).or_else(|| {
        let (subject, actor) = nearest_subject(words, start, end)?;
        coordinated_subject_actor(words, start, subject)
            .filter(|actor| *actor == SetupActor::Child)
            .or(Some(actor))
    })
}

pub(super) fn agents_fail_closed(words: &[&str], start: usize, end: usize) -> Option<SetupActor> {
    let mut saw_non_child = false;
    for index in start..end {
        if !actor_is_introduced_by(words, start, index) {
            continue;
        }
        match actor_word(words[index]) {
            Some(SetupActor::Child) => return Some(SetupActor::Child),
            Some(SetupActor::NonChild) => saw_non_child = true,
            None => {}
        }
    }
    saw_non_child.then_some(SetupActor::NonChild)
}

fn nearest_subject(words: &[&str], start: usize, end: usize) -> Option<(usize, SetupActor)> {
    (start..end).rev().find_map(|index| {
        (!actor_is_introduced_by(words, start, index)
            && !contrastive_actor(words, index)
            && !relative_clause_object(words, start, index)
            && !coordinated_predicate_object(words, start, index, end))
        .then(|| actor_word(words[index]).map(|actor| (index, actor)))
        .flatten()
    })
}

fn relative_clause_object(words: &[&str], start: usize, actor: usize) -> bool {
    let Some(relative) = words[start..actor]
        .iter()
        .rposition(|word| matches!(*word, "who" | "which" | "that"))
        .map(|offset| start + offset)
    else {
        return false;
    };
    !words[relative + 1..actor]
        .iter()
        .any(|word| report_clause_predicate(word))
}

fn report_clause_predicate(word: &str) -> bool {
    matches!(
        word,
        "reports"
            | "reported"
            | "says"
            | "said"
            | "states"
            | "stated"
            | "explains"
            | "explained"
            | "notes"
            | "noted"
            | "tells"
            | "told"
    )
}

fn coordinated_predicate_object(words: &[&str], start: usize, actor: usize, end: usize) -> bool {
    (start..actor).any(|index| actor_word(words[index]).is_some())
        && words[actor + 1..end]
            .windows(2)
            .any(|words| words == ["and", "then"])
}

fn contrastive_actor(words: &[&str], actor: usize) -> bool {
    words.get(actor.saturating_sub(1)) == Some(&"not")
        || (words.get(actor.saturating_sub(1)) == Some(&"the")
            && words.get(actor.saturating_sub(2)) == Some(&"not"))
}

fn coordinated_subject_actor(words: &[&str], start: usize, subject: usize) -> Option<SetupActor> {
    let conjunction = (start..subject)
        .rev()
        .find(|index| words[*index] == "and")?;
    let (previous, actor) = nearest_subject(words, start, conjunction)?;
    (words[previous + 1..conjunction]
        .iter()
        .chain(words[conjunction + 1..subject].iter())
        .all(|word| subject_modifier(word)))
    .then_some(actor)
}

fn together_with_subject_actor(words: &[&str], start: usize, end: usize) -> Option<SetupActor> {
    let together = (start..end).find(|index| words[*index] == "together")?;
    let with = together + 1;
    (words.get(with) == Some(&"with")).then_some(())?;
    let (first_index, first) = nearest_subject(words, start, together)?;
    let (second_index, second) =
        (with + 1..end).find_map(|index| actor_word(words[index]).map(|actor| (index, actor)))?;
    (words[first_index + 1..together]
        .iter()
        .chain(words[with + 1..second_index].iter())
        .all(|word| subject_modifier(word)))
    .then_some(if first == second {
        first
    } else {
        SetupActor::Child
    })
}

fn subject_modifier(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "the"
            | "this"
            | "that"
            | "its"
            | "our"
            | "owning"
            | "implementation"
            | "lane"
            | "owner"
            | "thread"
    )
}

fn actor_is_introduced_by(words: &[&str], start: usize, actor: usize) -> bool {
    words[start..actor]
        .iter()
        .rposition(|word| *word == "by")
        .is_some_and(|offset| {
            let by = start + offset;
            words[by + 1..actor]
                .iter()
                .all(|word| subject_modifier(word))
        })
}

fn actor_word(word: &str) -> Option<SetupActor> {
    match word {
        "child" => Some(SetupActor::Child),
        "parent" | "orchestrator" => Some(SetupActor::NonChild),
        _ => None,
    }
}
