use super::child_lane_ownership_phrases::{has_absent_field_value, trimmed_value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerDecision {
    Child,
    CurrentThread,
    Parent,
    External,
}

impl OwnerDecision {
    pub(super) fn parse(value: &str) -> Option<Self> {
        let value = trimmed_value(value);
        if has_negated_owner(value) || has_negated_child_routing_requirement(value) {
            return None;
        }
        let owners = [
            (Self::Child, is_child_delegation(value)),
            (
                Self::CurrentThread,
                has_owner_token(value, "current-thread-owned"),
            ),
            (Self::Parent, has_owner_token(value, "parent-owned")),
            (
                Self::External,
                has_owner_token(value, "external/human-owned"),
            ),
        ];
        let matches = owners
            .into_iter()
            .filter_map(|(owner, present)| present.then_some(owner))
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

fn is_child_delegation(value: &str) -> bool {
    has_owner_token(value, "child-owned")
        || ((value.contains("child delegation")
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
                || value.contains("tool-discovery-only")))
}

fn has_owner_token(value: &str, owner: &str) -> bool {
    value.match_indices(owner).any(|(index, _)| {
        let boundary = |byte: u8| !byte.is_ascii_alphanumeric() && byte != b'-';
        (index == 0 || boundary(value.as_bytes()[index - 1]))
            && value
                .as_bytes()
                .get(index + owner.len())
                .is_none_or(|byte| boundary(*byte))
    })
}

fn has_negated_owner(value: &str) -> bool {
    [
        "child-owned",
        "current-thread-owned",
        "parent-owned",
        "external/human-owned",
    ]
    .into_iter()
    .any(|owner| {
        value.match_indices(owner).any(|(index, _)| {
            has_owner_token(&value[index..], owner)
                && (value[..index]
                    .rsplit(|character| matches!(character, ',' | ';' | '.'))
                    .next()
                    .is_some_and(|clause| {
                        clause
                            .split(|character: char| !character.is_ascii_alphabetic())
                            .any(|word| matches!(word, "not" | "no" | "without"))
                    })
                    || (owner == "current-thread-owned"
                        && value.contains("현재 작업이 구현을 소유하지 않음")))
        })
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
