use super::super::change_impact::{AnalysisLimits, ImpactAnalysis};
use super::super::change_input::{
    ChangeKind, ChangeSet, FileChange, FileObservedState, ObservedState,
};
use super::model::{
    CheckDefinition, CheckMapping, CheckMappings, DependencyState, GapReason, ManualReason,
    MappingKind, MappingOwner, SelectionOptions,
};
use super::{recommend, recommend_with_options};

fn changes(paths: &[&str]) -> ChangeSet {
    ChangeSet {
        baseline_revision: "base".into(),
        observed_state: ObservedState::HeadComparison {
            requested_head: "head".into(),
            head_revision: "head".into(),
        },
        changes: paths
            .iter()
            .map(|path| FileChange {
                kind: ChangeKind::Modified,
                previous_path: Some((*path).into()),
                current_path: Some((*path).into()),
                observed_state: FileObservedState::HeadComparison,
            })
            .collect(),
    }
}

fn empty_impact() -> ImpactAnalysis {
    ImpactAnalysis {
        affected_files: Vec::new(),
        causal_changes: Vec::new(),
        connecting_paths: Vec::new(),
        limits: AnalysisLimits {
            max_files: 80,
            max_paths: 256,
            baseline_truncated: false,
            current_truncated: false,
            paths_truncated: false,
            unknown: Vec::new(),
        },
        partial: false,
    }
}

#[test]
fn contradictory_mapping_data_remains_visible() {
    let mappings = CheckMappings::new(
        vec![
            CheckDefinition {
                id: "lint".into(),
                command: "cargo fmt --check".into(),
                description: "format check".into(),
            },
            CheckDefinition {
                id: "lint".into(),
                command: "cargo clippy".into(),
                description: "lint check".into(),
            },
            CheckDefinition {
                id: "format".into(),
                command: "cargo fmt --check".into(),
                description: "format check".into(),
            },
        ],
        vec![
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["lint".into()],
                reason: "repository mapping".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["format".into()],
                reason: "contradictory repository mapping".into(),
            },
        ],
    );
    let result = recommend(&changes(&["src/lib.rs"]), &empty_impact(), &mappings);

    assert!(result.partial);
    assert!(result.recommendations.is_empty());
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| { gap.path.is_none() && gap.detail.contains("conflicting definitions") })
    );
    assert!(result.gaps.iter().any(|gap| {
        gap.mapping_pattern.as_deref() == Some("src/")
            && gap.reason == GapReason::ContradictoryMapping
    }));
    assert!(
        result
            .manual_judgment
            .iter()
            .any(|item| item.reason == ManualReason::ContradictoryMapping)
    );
}

#[test]
fn reordered_equivalent_mapping_sets_remain_recommendations() {
    let mappings = CheckMappings::new(
        vec![
            CheckDefinition {
                id: "fmt".into(),
                command: "cargo fmt --check".into(),
                description: "format check".into(),
            },
            CheckDefinition {
                id: "clippy".into(),
                command: "cargo clippy".into(),
                description: "lint check".into(),
            },
        ],
        vec![
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["fmt".into(), "clippy".into()],
                reason: "repository mapping".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["clippy".into(), "fmt".into(), "fmt".into()],
                reason: "equivalent repository mapping".into(),
            },
        ],
    );
    let result = recommend_with_options(
        &changes(&["src/lib.rs"]),
        &empty_impact(),
        &mappings,
        &SelectionOptions {
            dependency_state: DependencyState::Confirmed,
        },
    );

    assert!(!result.partial);
    assert!(result.gaps.is_empty());
    assert_eq!(
        result
            .recommendations
            .iter()
            .map(|recommendation| recommendation.id.as_str())
            .collect::<Vec<_>>(),
        ["clippy", "fmt"]
    );
}
