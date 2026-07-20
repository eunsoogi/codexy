use super::child_lane_ownership_phrases::{has_absent_field_value, trimmed_value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerDecision {
    Child,
    CurrentThread,
    Parent,
    External,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OwnerTruth {
    None,
    Affirmative,
    Negated,
    Contradictory,
}

impl OwnerDecision {
    pub(super) fn parse(value: &str) -> Option<Self> {
        let value = trimmed_value(value);
        let owners = [
            (Self::Child, owner_truth(value, "child-owned")),
            (Self::CurrentThread, current_thread_truth(value)),
            (Self::Parent, owner_truth(value, "parent-owned")),
            (Self::External, owner_truth(value, "external/human-owned")),
        ];
        if owners
            .iter()
            .any(|(_, truth)| *truth == OwnerTruth::Contradictory)
        {
            return None;
        }
        if is_child_routing_owner(value, owners[0].1) {
            return Some(Self::Child);
        }
        let matches = owners
            .into_iter()
            .filter_map(|(owner, truth)| (truth == OwnerTruth::Affirmative).then_some(owner))
            .collect::<Vec<_>>();
        (matches.len() == 1).then(|| matches[0])
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Child => "child-owned",
            Self::CurrentThread => "current-thread-owned",
            Self::Parent => "parent-owned",
            Self::External => "external/human-owned",
        }
    }

    pub(super) fn is_child(self) -> bool {
        matches!(self, Self::Child | Self::CurrentThread)
    }
}

pub(super) fn is_child_delegation_owner_decision(value: &str) -> bool {
    OwnerDecision::parse(value).is_some_and(OwnerDecision::is_child)
}

pub(super) fn is_affirmative_child_owned_value(value: &str) -> bool {
    OwnerDecision::parse(value) == Some(OwnerDecision::Child)
        && !has_absent_field_value(trimmed_value(value), "child-owned")
}

pub(super) fn is_parent_owned_value(value: &str) -> bool {
    OwnerDecision::parse(value) == Some(OwnerDecision::Parent)
}

fn is_child_routing_owner(value: &str, child: OwnerTruth) -> bool {
    matches!(child, OwnerTruth::None | OwnerTruth::Affirmative)
        && !has_negated_child_routing_requirement(value)
        && (value.contains("child delegation")
            || value.contains("child-lane setup")
            || value.contains("child routing")
            || value.contains("child thread/worktree owner")
            || value.contains("thread/worktree tool discovery")
            || value.contains("thread tool discovery")
            || value.contains("worktree tool discovery"))
        && (value.contains("routing-only")
            || value.contains("coordination-only")
            || value.contains("delegation-only")
            || value.contains("child routing required")
            || value.contains("owner required")
            || value.contains("tool discovery only")
            || value.contains("tool-discovery-only"))
}

fn owner_truth(value: &str, owner: &str) -> OwnerTruth {
    let (affirmative, negated) =
        value
            .match_indices(owner)
            .fold((false, false), |state, (index, _)| {
                if !is_owner_token_at(value, owner, index) {
                    state
                } else if is_negated_owner_token(value, index) {
                    (state.0, true)
                } else {
                    (true, state.1)
                }
            });
    match (affirmative, negated) {
        (false, false) => OwnerTruth::None,
        (true, false) => OwnerTruth::Affirmative,
        (false, true) => OwnerTruth::Negated,
        (true, true) => OwnerTruth::Contradictory,
    }
}

fn current_thread_truth(value: &str) -> OwnerTruth {
    let truth = owner_truth(value, "current-thread-owned");
    let korean = match (
        value.contains("현재 작업이 구현을 소유함"),
        value.contains("현재 작업이 구현을 소유하지 않음"),
    ) {
        (false, false) => OwnerTruth::None,
        (true, false) => OwnerTruth::Affirmative,
        (false, true) => OwnerTruth::Negated,
        (true, true) => OwnerTruth::Contradictory,
    };
    match (truth, korean) {
        (OwnerTruth::None, value) | (value, OwnerTruth::None) => value,
        (OwnerTruth::Affirmative, OwnerTruth::Affirmative) => OwnerTruth::Affirmative,
        (OwnerTruth::Negated, OwnerTruth::Negated) => OwnerTruth::Negated,
        _ => OwnerTruth::Contradictory,
    }
}

fn is_owner_token_at(value: &str, owner: &str, index: usize) -> bool {
    let boundary = |byte: u8| !byte.is_ascii_alphanumeric() && byte != b'-';
    (index == 0 || boundary(value.as_bytes()[index - 1]))
        && value
            .as_bytes()
            .get(index + owner.len())
            .is_none_or(|byte| boundary(*byte))
}

fn is_negated_owner_token(value: &str, index: usize) -> bool {
    value[..index]
        .rsplit(|character| matches!(character, ',' | ';' | '.'))
        .next()
        .is_some_and(|clause| {
            clause
                .split(|character: char| !character.is_ascii_alphabetic())
                .any(|word| matches!(word, "not" | "no" | "without"))
        })
}

fn has_negated_child_routing_requirement(value: &str) -> bool {
    let value = value.replace("child-routing", "child routing");
    [
        "no child routing required",
        "child routing not required",
        "no child routing is required",
        "child routing is not required",
        "without child routing",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}
