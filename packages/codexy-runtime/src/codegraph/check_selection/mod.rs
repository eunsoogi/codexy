mod classify;
mod concerns;
mod model;
mod selection;
mod validation;

#[cfg(test)]
#[path = "classify_tests.rs"]
mod classify_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
#[path = "mapping_tests.rs"]
mod mapping_tests;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use super::change_impact::{ImpactAnalysis, ImpactLevel};
use super::change_input::{ChangeSet, FileChange};
use classify::normalize_path;
use model::PathEvidence;

pub use classify::classify_path;
pub use model::{
    BroaderReason, BroaderVerification, ChangeArea, CheckDefinition, CheckMapping, CheckMappings,
    CheckRecommendation, CheckSelection, DependencyState, GapReason, ManualJudgment, ManualReason,
    MappingKind, MappingOwner, SelectionGap, SelectionOptions,
};

/// Recommend checks from explicit path/configuration/fixture mappings and the
/// read-only change and impact results.
///
/// Mapping commands are returned as data. They are never parsed, spawned, or
/// otherwise executed. Missing mappings, missing check definitions, and
/// uncertain impact remain visible in the result.
#[must_use]
pub fn recommend(
    changes: &ChangeSet,
    impact: &ImpactAnalysis,
    mappings: &CheckMappings,
) -> CheckSelection {
    let options = SelectionOptions::default();
    recommend_with_options(changes, impact, mappings, &options)
}

/// Recommend checks with explicit dependency evidence.
#[must_use]
pub fn recommend_with_options(
    changes: &ChangeSet,
    impact: &ImpactAnalysis,
    mappings: &CheckMappings,
    options: &SelectionOptions,
) -> CheckSelection {
    let paths = collect_paths(changes, impact);
    let selected = selection::select(&paths, impact, mappings, options);
    CheckSelection {
        baseline_revision: changes.baseline_revision.clone(),
        observed_state: changes.observed_state.clone(),
        recommendations: selected.recommendations,
        gaps: selected.gaps,
        broader_verification: selected.broader_verification,
        manual_judgment: selected.manual_judgment,
        impact_unknown: impact.limits.unknown.clone(),
        partial: selected.partial,
    }
}

fn collect_paths(changes: &ChangeSet, impact: &ImpactAnalysis) -> BTreeMap<String, PathEvidence> {
    let mut paths = BTreeMap::new();
    for change in &changes.changes {
        for path in change_paths(change) {
            add_path(&mut paths, &path, ImpactLevel::Direct);
        }
    }
    for affected in &impact.affected_files {
        add_path(&mut paths, &affected.path, affected.impact);
    }
    paths
}

fn change_paths(change: &FileChange) -> BTreeSet<String> {
    change
        .previous_path
        .iter()
        .chain(change.current_path.iter())
        .map(|path| normalize_path(path))
        .collect()
}

fn add_path(paths: &mut BTreeMap<String, PathEvidence>, path: &str, impact: ImpactLevel) {
    let path = normalize_path(path);
    let area = classify_path(&path);
    let entry = paths.entry(path).or_insert(PathEvidence { area, impact });
    entry.impact = merge_impact(entry.impact, impact);
}

const fn merge_impact(left: ImpactLevel, right: ImpactLevel) -> ImpactLevel {
    match (left, right) {
        (ImpactLevel::Unknown, _) | (_, ImpactLevel::Unknown) => ImpactLevel::Unknown,
        (ImpactLevel::Direct, _) | (_, ImpactLevel::Direct) => ImpactLevel::Direct,
        _ => ImpactLevel::Transitive,
    }
}
