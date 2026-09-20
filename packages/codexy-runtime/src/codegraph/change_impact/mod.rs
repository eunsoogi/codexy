mod git;
mod graph;
mod model;

#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Result;

use super::change_input::{ChangeSet, ObservedState};
use graph::{SnapshotGraph, is_supported_path};
use model::AffectedAccumulator;

pub use model::{
    AffectedFile, AnalysisLimits, ImpactAnalysis, ImpactLevel, ImpactOptions, ImpactPath,
    UnknownArea, UnknownReason,
};

/// Analyze the direct and transitive impact of one read-only change set.
///
/// Baseline revisions are read through `git archive`; the working tree is
/// never checked out, reset, or otherwise mutated. Only Python and Rust
/// dependency paths are treated as known.
///
/// # Errors
///
/// Returns an error when the baseline or requested head cannot be archived.
pub fn analyze(root: &Path, changes: &ChangeSet) -> Result<ImpactAnalysis> {
    analyze_with_options(root, changes, ImpactOptions::default())
}

/// Analyze a change set with explicit file and causal-path limits.
///
/// Limits are part of the result. Exceeding one records an unknown area rather
/// than claiming that omitted files have no impact.
///
/// # Errors
///
/// Returns an error when the baseline or requested head cannot be archived.
pub fn analyze_with_options(
    root: &Path,
    changes: &ChangeSet,
    options: ImpactOptions,
) -> Result<ImpactAnalysis> {
    let options = options.normalized();
    let baseline = git::snapshot_revision(root, &changes.baseline_revision, options.max_files)?;
    let current = match &changes.observed_state {
        ObservedState::HeadComparison { head_revision, .. } => {
            git::snapshot_revision(root, head_revision, options.max_files)?
        }
        ObservedState::WorkingTree { .. } => git::snapshot_current(root, options.max_files),
    };
    let mut limits = AnalysisLimits {
        max_files: options.max_files,
        max_paths: options.max_paths,
        baseline_truncated: baseline.truncated,
        current_truncated: current.truncated,
        paths_truncated: false,
        unknown: Vec::new(),
    };
    let mut output = AffectedAccumulator::default();
    record_snapshot_limit(&mut output, &baseline, "baseline");
    record_snapshot_limit(&mut output, &current, "current");

    for (change_index, change) in changes.changes.iter().enumerate() {
        for change_path in change_paths(change) {
            output.add_file(&change_path, ImpactLevel::Direct, change_index);
            add_path(
                &mut output,
                ImpactPath {
                    change_index,
                    change_path: change_path.clone(),
                    affected_path: change_path.clone(),
                    path: vec![change_path.clone()],
                },
            );
            if !is_supported_path(&change_path) {
                output.add_file(&change_path, ImpactLevel::Unknown, change_index);
                output.add_unknown(UnknownArea {
                    reason: UnknownReason::UnsupportedLanguage,
                    path: Some(change_path.clone()),
                    detail: "impact analysis supports Python and Rust only".to_owned(),
                });
                continue;
            }
            for snapshot in [&baseline, &current] {
                record_path_unknowns(&mut output, snapshot, &change_path, change_index);
                let paths = snapshot.reverse_paths(&change_path, options.max_paths);
                limits.paths_truncated |= paths.truncated;
                if paths.truncated {
                    output.add_unknown(UnknownArea {
                        reason: UnknownReason::PathLimit,
                        path: Some(change_path.clone()),
                        detail: format!("causal paths exceeded the limit of {}", options.max_paths),
                    });
                }
                for path in paths.paths {
                    let Some(affected_path) = path.last().cloned() else {
                        continue;
                    };
                    output.add_file(&affected_path, ImpactLevel::Transitive, change_index);
                    for path_node in &path {
                        record_path_unknowns(&mut output, snapshot, path_node, change_index);
                    }
                    add_path(
                        &mut output,
                        ImpactPath {
                            change_index,
                            change_path: change_path.clone(),
                            affected_path,
                            path,
                        },
                    );
                }
            }
        }
    }
    Ok(output.finish(changes.changes.clone(), limits))
}

fn change_paths(change: &super::change_input::FileChange) -> Vec<String> {
    change
        .previous_path
        .iter()
        .chain(change.current_path.iter())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn record_snapshot_limit(output: &mut AffectedAccumulator, snapshot: &SnapshotGraph, side: &str) {
    if snapshot.truncated {
        output.add_unknown(UnknownArea {
            reason: UnknownReason::FileLimit,
            path: None,
            detail: format!("{side} snapshot exceeded the configured file limit"),
        });
    }
}

fn record_path_unknowns(
    output: &mut AffectedAccumulator,
    snapshot: &SnapshotGraph,
    path: &str,
    change_index: usize,
) {
    for area in snapshot.unknown_for_path(path) {
        if let Some(unknown_path) = area.path.as_deref() {
            output.add_file(unknown_path, ImpactLevel::Unknown, change_index);
        }
        output.add_unknown(area);
    }
}

fn add_path(output: &mut AffectedAccumulator, path: ImpactPath) {
    if !output.paths.iter().any(|existing| existing == &path) {
        output.paths.push(path);
    }
}
