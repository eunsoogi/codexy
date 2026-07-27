use super::child_lane_classification_setup_clause::SENTENCE_BOUNDARY;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum SetupActor {
    Child,
    NonChild,
}

pub(super) fn explicit_subject(words: &[&str], start: usize, action: usize) -> Option<SetupActor> {
    together_with_subject_actor(words, start, action).or_else(|| {
        let (subject, subject_start) = clause_subject(words, start, action)
            .map(|subject| (subject, start))
            .or_else(|| {
                let inherited_start = words[..start]
                    .iter()
                    .rposition(|word| {
                        *word == SENTENCE_BOUNDARY || matches!(*word, "but" | "however")
                    })
                    .map(|index| index + 1)
                    .unwrap_or(0);
                clause_subject(words, inherited_start, action)
                    .map(|subject| (subject, inherited_start))
            })?;
        mixed_coordinated_subject(words, subject_start, subject.index)
            .unwrap_or(subject.actor)
            .into()
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

#[derive(Clone, Copy)]
struct ClauseSubject {
    index: usize,
    actor: SetupActor,
}

fn clause_subject(words: &[&str], start: usize, action: usize) -> Option<ClauseSubject> {
    let mut subject = initial_clause_subject(words, start, action)?;
    for index in subject.index + 1..action {
        if predicate_subject(words, start, index, action) {
            subject = ClauseSubject {
                index,
                actor: actor_word(words[index])?,
            };
        }
    }
    Some(subject)
}

fn initial_clause_subject(words: &[&str], start: usize, action: usize) -> Option<ClauseSubject> {
    (start..action).find_map(|index| {
        (actor_word(words[index]).is_some()
            && !actor_is_introduced_by(words, start, index)
            && !contrastive_actor(words, index)
            && words[start..index]
                .iter()
                .all(|word| subject_modifier(word)))
        .then(|| actor_word(words[index]).map(|actor| ClauseSubject { index, actor }))
        .flatten()
    })
}

fn predicate_subject(words: &[&str], start: usize, index: usize, action: usize) -> bool {
    actor_word(words[index]).is_some()
        && !contrastive_actor(words, index)
        && predicate_tail(words, index, action)
        && predicate_introducer(words, start, index)
}

fn predicate_tail(words: &[&str], subject: usize, action: usize) -> bool {
    words[subject + 1..action]
        .iter()
        .all(|word| subject_modifier(word) || predicate_modifier(word))
}

fn predicate_introducer(words: &[&str], start: usize, subject: usize) -> bool {
    let Some(index) = (start..subject)
        .rev()
        .find(|index| !subject_modifier(words[*index]))
    else {
        return false;
    };
    words[index] == "and"
        || (report_clause_predicate(words[index])
            && !relative_clause_owns_report_predicate(words, start, index))
        || (words[index] == "then"
            && (start..index)
                .rev()
                .find(|index| !subject_modifier(words[*index]))
                == Some(index - 1)
            && words[index - 1] == "and")
}

fn relative_clause_owns_report_predicate(words: &[&str], start: usize, predicate: usize) -> bool {
    let Some(relative) = (start..predicate)
        .rev()
        .find(|index| matches!(words[*index], "who" | "whose" | "which"))
    else {
        return false;
    };
    let clause_start = if words[relative] == "whose" {
        (relative + 1..predicate)
            .find(|index| !subject_modifier(words[*index]))
            .map_or(predicate, |subject| subject + 1)
    } else {
        relative + 1
    };
    words[clause_start..predicate]
        .iter()
        .all(|word| subject_modifier(word) || predicate_modifier(word))
}

fn mixed_coordinated_subject(words: &[&str], start: usize, subject: usize) -> Option<SetupActor> {
    let conjunction = (start..subject)
        .rev()
        .find(|index| !subject_modifier(words[*index]))?;
    (words[conjunction] == "and").then_some(())?;
    let previous = initial_clause_subject(words, start, conjunction)?;
    (words[previous.index + 1..conjunction]
        .iter()
        .all(|word| subject_modifier(word))
        && previous.actor != actor_word(words[subject])?)
    .then_some(SetupActor::Child)
}

fn contrastive_actor(words: &[&str], actor: usize) -> bool {
    words.get(actor.saturating_sub(1)) == Some(&"not")
        || (words.get(actor.saturating_sub(1)) == Some(&"the")
            && words.get(actor.saturating_sub(2)) == Some(&"not"))
}

fn together_with_subject_actor(words: &[&str], start: usize, end: usize) -> Option<SetupActor> {
    let together = (start..end).find(|index| words[*index] == "together")?;
    let with = together + 1;
    (words.get(with) == Some(&"with")).then_some(())?;
    let first = initial_clause_subject(words, start, together)?;
    let (second_index, second) =
        (with + 1..end).find_map(|index| actor_word(words[index]).map(|actor| (index, actor)))?;
    (words[first.index + 1..together]
        .iter()
        .chain(words[with + 1..second_index].iter())
        .all(|word| subject_modifier(word)))
    .then_some(if first.actor == second {
        first.actor
    } else {
        SetupActor::Child
    })
}

fn predicate_modifier(word: &str) -> bool {
    word.ends_with("ly")
        || matches!(
            word,
            "is" | "are"
                | "was"
                | "were"
                | "be"
                | "been"
                | "being"
                | "has"
                | "have"
                | "had"
                | "will"
                | "would"
                | "can"
                | "could"
                | "may"
                | "might"
                | "should"
                | "must"
                | "do"
                | "does"
                | "did"
                | "not"
                | "never"
        )
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
