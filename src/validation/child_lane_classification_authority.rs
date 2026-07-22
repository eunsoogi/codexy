use super::child_lane_owner_decision::OwnerSelection;

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

pub(super) fn lane_authority_before(
    lines: &[&str],
    classification_start: usize,
) -> Option<LaneAuthority> {
    if lines.get(classification_start) != Some(&"task classification:") {
        return None;
    }
    let (source, ownership) = classification_start
        .checked_sub(2)
        .and_then(|start| lines.get(start..classification_start))
        .and_then(|metadata| metadata.first().zip(metadata.get(1)))?;
    let source = match *source {
        "ownership metadata source: parent-supplied" => AuthoritySource::ParentSupplied,
        "ownership metadata source: current-thread-classified" => {
            AuthoritySource::CurrentThreadClassified
        }
        _ => return None,
    };
    let owner = match *ownership {
        "lane ownership: parent-owned" => OwnerSelection::ParentOwned,
        "lane ownership: child-owned" => OwnerSelection::ChildOwned,
        "lane ownership: current-thread-owned" => OwnerSelection::CurrentThreadOwned,
        "lane ownership: external/human-owned" => OwnerSelection::ExternalHumanOwned,
        _ => return None,
    };
    Some(LaneAuthority { owner, source })
}
