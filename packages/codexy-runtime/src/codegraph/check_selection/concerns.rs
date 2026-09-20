use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    BroaderReason, BroaderVerification, ChangeArea, CheckMapping, ManualJudgment, ManualReason,
};

pub(super) fn add_area_concern(
    concerns: &mut BTreeMap<BroaderReason, BTreeSet<String>>,
    area: &ChangeArea,
    path: &str,
) {
    let reason = match area {
        ChangeArea::SharedFixture => Some(BroaderReason::SharedFixture),
        ChangeArea::Lockfile => Some(BroaderReason::Lockfile),
        ChangeArea::Configuration => Some(BroaderReason::Configuration),
        ChangeArea::Documentation | ChangeArea::Module | ChangeArea::Other => None,
    };
    if let Some(reason) = reason {
        add_path_concern(concerns, reason, path);
    }
}

pub(super) fn add_path_concern<K>(
    concerns: &mut BTreeMap<K, BTreeSet<String>>,
    reason: K,
    path: &str,
) where
    K: Ord,
{
    concerns.entry(reason).or_default().insert(path.to_owned());
}

pub(super) fn add_global_concern<K>(concerns: &mut BTreeMap<K, BTreeSet<String>>, reason: K)
where
    K: Ord,
{
    concerns.entry(reason).or_default();
}

pub(super) fn finish_broader(
    concerns: BTreeMap<BroaderReason, BTreeSet<String>>,
) -> Vec<BroaderVerification> {
    concerns
        .into_iter()
        .map(|(reason, values)| BroaderVerification {
            reason,
            paths: values.into_iter().collect(),
            detail: broader_detail(reason).to_owned(),
        })
        .collect()
}

pub(super) fn finish_manual(
    concerns: BTreeMap<ManualReason, BTreeSet<String>>,
) -> Vec<ManualJudgment> {
    concerns
        .into_iter()
        .map(|(reason, paths)| ManualJudgment {
            reason,
            paths: paths.into_iter().collect(),
            detail: manual_detail(reason).to_owned(),
        })
        .collect()
}

const fn broader_detail(reason: BroaderReason) -> &'static str {
    match reason {
        BroaderReason::SharedFixture => "shared fixtures can affect multiple consumers",
        BroaderReason::Lockfile => "lockfile changes can alter resolved dependencies",
        BroaderReason::Configuration => "configuration changes can affect multiple checks",
        BroaderReason::PartialImpact => "impact analysis is partial; broaden verification",
        BroaderReason::UnknownImpact => "impact is unknown; do not infer no impact",
        BroaderReason::UnconfirmedDependencies => "dependency state was not confirmed",
    }
}

const fn manual_detail(reason: ManualReason) -> &'static str {
    match reason {
        ManualReason::MissingMapping => "an owner must complete the explicit path mapping",
        ManualReason::MissingCheck => "an owner must complete the referenced check inventory",
        ManualReason::ContradictoryMapping => {
            "an owner must resolve the conflicting check mapping data"
        }
        ManualReason::PartialAnalysis => "an owner must judge whether broader checks are needed",
        ManualReason::UnknownImpact => "an owner must review the unresolved impact scope",
        ManualReason::UnconfirmedDependencies => "an owner must confirm dependency impact",
    }
}

pub(super) const fn area_label(area: &ChangeArea) -> &'static str {
    match area {
        ChangeArea::Documentation => "documentation",
        ChangeArea::Module => "module",
        ChangeArea::SharedFixture => "shared-fixture",
        ChangeArea::Lockfile => "lockfile",
        ChangeArea::Configuration => "configuration",
        ChangeArea::Other => "other",
    }
}

pub(super) const fn owner_label(mapping: &CheckMapping) -> &'static str {
    match mapping.owner {
        super::model::MappingOwner::User => "user",
        super::model::MappingOwner::Repository => "repository",
    }
}

pub(super) const fn mapping_kind_label(mapping: &CheckMapping) -> &'static str {
    match mapping.kind {
        super::model::MappingKind::Path => "path",
        super::model::MappingKind::SharedConfiguration => "shared-configuration",
        super::model::MappingKind::Fixture => "fixture",
    }
}
