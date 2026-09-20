use super::super::change_impact::{AffectedFile, AnalysisLimits, ImpactAnalysis, ImpactLevel};
use super::super::change_input::{
    ChangeKind, ChangeSet, FileChange, FileObservedState, ObservedState,
};
use super::fixtures::supported_examples;
use super::model::{
    ChangeArea, CheckDefinition, CheckMapping, CheckMappings, DependencyState, GapReason,
    ManualReason, MappingKind, MappingOwner, SelectionOptions,
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

fn impact(paths: &[(&str, ImpactLevel)], partial: bool) -> ImpactAnalysis {
    ImpactAnalysis {
        affected_files: paths
            .iter()
            .map(|(path, impact)| AffectedFile {
                path: (*path).into(),
                impact: *impact,
                causal_change_indices: vec![0],
            })
            .collect(),
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
        partial,
    }
}

#[test]
fn categories_select_distinguishable_supported_checks() {
    let changes = changes(&[
        "docs/guide.md",
        "packages/codexy-runtime/src/codegraph/check_selection/mod.rs",
        "packages/codexy-runtime/tests/fixtures/shared.rs",
        "Cargo.lock",
    ]);
    let result = recommend(&changes, &impact(&[], false), &supported_examples());

    let ids = result
        .recommendations
        .iter()
        .map(|recommendation| recommendation.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["codegraph", "docs", "runtime"]);
    assert!(
        result.recommendations[0]
            .areas
            .contains(&ChangeArea::Module)
    );
    assert!(
        result.recommendations[1]
            .areas
            .contains(&ChangeArea::Documentation)
    );
    assert!(
        result.recommendations[2]
            .areas
            .contains(&ChangeArea::SharedFixture)
    );
    assert!(
        result.recommendations[2]
            .areas
            .contains(&ChangeArea::Lockfile)
    );
    assert!(
        result
            .broader_verification
            .iter()
            .any(|item| item.reason == super::model::BroaderReason::Lockfile)
    );
    assert!(result.gaps.is_empty());
}

#[test]
fn missing_mapping_and_unknown_impact_remain_visible() {
    let changes = changes(&["packages/unknown/odd.data"]);
    let mut impact = impact(&[("packages/unknown/odd.data", ImpactLevel::Unknown)], true);
    impact
        .limits
        .unknown
        .push(super::super::change_impact::UnknownArea {
            reason: super::super::change_impact::UnknownReason::ParseFailure,
            path: Some("packages/unknown/odd.data".into()),
            detail: "parser did not provide a complete result".into(),
        });
    let result = recommend(&changes, &impact, &CheckMappings::default());

    assert!(result.partial);
    assert!(result.gaps.iter().any(|gap| {
        gap.reason == GapReason::MissingMapping
            && gap.path.as_deref() == Some("packages/unknown/odd.data")
    }));
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.reason == GapReason::UnknownImpact)
    );
    assert_eq!(result.impact_unknown.len(), 1);
    assert!(
        result
            .manual_judgment
            .iter()
            .any(|item| item.reason == ManualReason::UnknownImpact)
    );
}

#[test]
fn lock_and_unconfirmed_dependency_changes_require_broader_judgment() {
    let mappings = supported_examples();
    let result = recommend_with_options(
        &changes(&["Cargo.lock"]),
        &impact(&[], false),
        &mappings,
        &SelectionOptions {
            dependency_state: DependencyState::Unconfirmed {
                detail: "dependency metadata was not observed".into(),
            },
        },
    );

    assert!(
        result
            .broader_verification
            .iter()
            .any(|item| item.reason == super::model::BroaderReason::Lockfile)
    );
    assert!(
        result
            .broader_verification
            .iter()
            .any(|item| item.reason == super::model::BroaderReason::UnconfirmedDependencies)
    );
    assert!(
        result
            .manual_judgment
            .iter()
            .any(|item| item.reason == ManualReason::UnconfirmedDependencies)
    );
    assert!(result.partial);
}

#[test]
fn incomplete_mappings_do_not_silently_drop_checks() {
    let mappings = CheckMappings::new(
        vec![CheckDefinition {
            id: "empty".into(),
            command: String::new(),
            description: "missing command".into(),
        }],
        vec![
            CheckMapping {
                owner: MappingOwner::User,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["missing".into()],
                reason: "user mapping".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "src/".into(),
                check_ids: vec!["empty".into()],
                reason: "repository mapping".into(),
            },
        ],
    );
    let result = recommend(&changes(&["src/lib.rs"]), &impact(&[], false), &mappings);

    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.reason == GapReason::MissingCheck)
    );
    assert!(
        result
            .gaps
            .iter()
            .any(|gap| gap.reason == GapReason::MissingCommand)
    );
    assert!(result.recommendations.is_empty());
}

#[test]
fn hostile_paths_and_commands_are_returned_as_data() {
    let marker = std::env::temp_dir().join(format!(
        "codexy-check-selection-marker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos()
    ));
    assert!(!marker.exists());
    let hostile_command = format!("$(touch {})", marker.display());
    let hostile_path = "../../outside/$(touch marker).rs";
    let mappings = CheckMappings::new(
        vec![CheckDefinition {
            id: "hostile".into(),
            command: hostile_command.clone(),
            description: "untrusted command text".into(),
        }],
        vec![CheckMapping {
            owner: MappingOwner::User,
            kind: MappingKind::Path,
            pattern: hostile_path.into(),
            check_ids: vec!["hostile".into()],
            reason: "untrusted path text".into(),
        }],
    );
    let result = recommend(&changes(&[hostile_path]), &impact(&[], false), &mappings);

    assert_eq!(result.recommendations.len(), 1);
    assert_eq!(result.recommendations[0].command, hostile_command);
    assert_eq!(result.recommendations[0].paths, [hostile_path]);
    assert!(!marker.exists(), "analysis executed the untrusted command");
    assert!(
        result.recommendations[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("user path mapping"))
    );
}
