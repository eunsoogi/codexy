use super::child_lane_classification_boundaries::{current_lane_record_start, lane_boundary};
use super::child_lane_owner_decision::{
    LaneOwnershipMetadata, OwnerSelection, parse_lane_ownership_metadata,
};
use super::child_lane_ownership_phrases::{metadata_key, trimmed_value};

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

#[derive(Clone, Copy)]
pub(super) enum AuthorityRecordAction {
    Control,
    Setup { explicit_child_scope: bool },
}

#[derive(Clone, Copy)]
pub(super) enum LaneAuthorityRecordState {
    Absent,
    Incomplete,
    Invalid,
    Complete(LaneAuthority),
}

enum AuthorityRecordBuildState {
    Absent,
    Source(AuthoritySource),
    Owner(LaneAuthority),
    Invalid,
    Complete(LaneAuthority),
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
    fn new(source: AuthoritySource, owner: OwnerSelection) -> Option<Self> {
        matches!(
            (source, owner),
            (AuthoritySource::ParentSupplied, OwnerSelection::ChildOwned)
                | (AuthoritySource::CurrentThreadClassified, _)
        )
        .then_some(Self { owner, source })
    }

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

    fn is_non_child_owner(self) -> bool {
        matches!(
            self.owner,
            OwnerSelection::ParentOwned | OwnerSelection::ExternalHumanOwned
        )
    }
}

impl LaneAuthorityRecordState {
    pub(super) fn validation_applies(
        self,
        classification_complete: bool,
        action: AuthorityRecordAction,
    ) -> Option<bool> {
        let authority = match self {
            Self::Absent => return None,
            Self::Incomplete | Self::Invalid => return Some(true),
            Self::Complete(_) if !classification_complete => return Some(true),
            Self::Complete(authority) => authority,
        };
        if authority.authorizes_child_setup() || !authority.is_non_child_owner() {
            return Some(true);
        }
        Some(match action {
            AuthorityRecordAction::Control => false,
            AuthorityRecordAction::Setup {
                explicit_child_scope,
            } => explicit_child_scope,
        })
    }
}

pub(super) fn lane_authority_record_state_before(
    lines: &[&str],
    end: usize,
) -> LaneAuthorityRecordState {
    let mut state = AuthorityRecordBuildState::Absent;
    for (index, line) in lines
        .iter()
        .enumerate()
        .take(end)
        .skip(current_lane_record_start(lines, end))
    {
        let line = trimmed_value(line);
        let normalized = metadata_key(line);
        if lane_boundary(lines, index).is_some_and(|boundary| boundary.resets_authority_record()) {
            state = AuthorityRecordBuildState::Absent;
        } else if normalized.starts_with("ownership metadata source:") {
            state = parse_authority_source(line).map_or(
                AuthorityRecordBuildState::Invalid,
                AuthorityRecordBuildState::Source,
            );
        } else if normalized.starts_with("lane ownership:") {
            state = match (state, parse_lane_ownership_metadata(line)) {
                (
                    AuthorityRecordBuildState::Source(source),
                    LaneOwnershipMetadata::Valid(owner),
                ) => LaneAuthority::new(source, owner).map_or(
                    AuthorityRecordBuildState::Invalid,
                    AuthorityRecordBuildState::Owner,
                ),
                _ => AuthorityRecordBuildState::Invalid,
            };
        } else if normalized == "task classification:" {
            state = match (line, state) {
                ("task classification:", AuthorityRecordBuildState::Owner(authority)) => {
                    AuthorityRecordBuildState::Complete(authority)
                }
                _ => AuthorityRecordBuildState::Invalid,
            };
        }
    }
    match state {
        AuthorityRecordBuildState::Absent => LaneAuthorityRecordState::Absent,
        AuthorityRecordBuildState::Source(_) | AuthorityRecordBuildState::Owner(_) => {
            LaneAuthorityRecordState::Incomplete
        }
        AuthorityRecordBuildState::Invalid => LaneAuthorityRecordState::Invalid,
        AuthorityRecordBuildState::Complete(authority) => {
            LaneAuthorityRecordState::Complete(authority)
        }
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
    let Some(source) = parse_authority_source(source) else {
        return LaneAuthorityContext {
            state: LaneAuthorityState::Invalid,
        };
    };
    let state = match parse_lane_ownership_metadata(ownership) {
        LaneOwnershipMetadata::Absent => LaneAuthorityState::Absent,
        LaneOwnershipMetadata::Invalid => LaneAuthorityState::Invalid,
        LaneOwnershipMetadata::Valid(owner) => LaneAuthority::new(source, owner)
            .map_or(LaneAuthorityState::Invalid, LaneAuthorityState::Valid),
    };
    LaneAuthorityContext { state }
}

fn parse_authority_source(line: &str) -> Option<AuthoritySource> {
    match line {
        "ownership metadata source: parent-supplied" => Some(AuthoritySource::ParentSupplied),
        "ownership metadata source: current-thread-classified" => {
            Some(AuthoritySource::CurrentThreadClassified)
        }
        _ => None,
    }
}
