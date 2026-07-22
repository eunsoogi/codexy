use super::child_lane_owner_decision::{
    LaneOwnershipMetadata, OwnerSelection, parse_lane_ownership_metadata,
};

#[derive(Clone, Copy)]
enum AuthoritySource {
    ParentSupplied,
    CurrentThreadClassified,
}

#[derive(Clone, Copy)]
pub(super) struct LaneAuthority {
    owner: OwnerSelection,
    source: AuthoritySource,
}

pub(super) struct LaneAuthorityContext {
    state: LaneAuthorityState,
}

enum LaneAuthorityState {
    Absent,
    Invalid,
    Valid(LaneAuthority),
}

impl LaneAuthorityContext {
    pub(super) fn authority(&self) -> Option<LaneAuthority> {
        match self.state {
            LaneAuthorityState::Valid(authority) => Some(authority),
            LaneAuthorityState::Absent | LaneAuthorityState::Invalid => None,
        }
    }
}

impl LaneAuthority {
    pub(super) fn owner(self) -> OwnerSelection {
        self.owner
    }

    pub(super) fn authorizes_child_setup(self) -> bool {
        matches!(
            (self.source, self.owner),
            (AuthoritySource::ParentSupplied, OwnerSelection::ChildOwned)
                | (
                    AuthoritySource::CurrentThreadClassified,
                    OwnerSelection::ChildOwned
                )
                | (
                    AuthoritySource::CurrentThreadClassified,
                    OwnerSelection::CurrentThreadOwned
                )
        )
    }
}

pub(super) fn lane_authority_context_before(
    lines: &[&str],
    classification_start: usize,
) -> LaneAuthorityContext {
    if lines.get(classification_start) != Some(&"task classification:") {
        return LaneAuthorityContext {
            state: LaneAuthorityState::Absent,
        };
    }
    let Some((source, ownership)) = classification_start
        .checked_sub(2)
        .and_then(|start| lines.get(start..classification_start))
        .and_then(|metadata| metadata.first().zip(metadata.get(1)))
    else {
        return LaneAuthorityContext {
            state: LaneAuthorityState::Absent,
        };
    };
    let source = match *source {
        "ownership metadata source: parent-supplied" => AuthoritySource::ParentSupplied,
        "ownership metadata source: current-thread-classified" => {
            AuthoritySource::CurrentThreadClassified
        }
        _ => {
            return LaneAuthorityContext {
                state: LaneAuthorityState::Invalid,
            };
        }
    };
    let state = match parse_lane_ownership_metadata(ownership) {
        LaneOwnershipMetadata::Absent => LaneAuthorityState::Absent,
        LaneOwnershipMetadata::Invalid => LaneAuthorityState::Invalid,
        LaneOwnershipMetadata::Valid(owner) => {
            LaneAuthorityState::Valid(LaneAuthority { owner, source })
        }
    };
    LaneAuthorityContext { state }
}
