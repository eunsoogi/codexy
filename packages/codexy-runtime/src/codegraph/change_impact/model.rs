use serde::Serialize;

use super::super::change_input::FileChange;

const DEFAULT_MAX_FILES: usize = 80;
const DEFAULT_MAX_PATHS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactOptions {
    pub max_files: usize,
    pub max_paths: usize,
}

impl Default for ImpactOptions {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
            max_paths: DEFAULT_MAX_PATHS,
        }
    }
}

impl ImpactOptions {
    pub(super) fn normalized(self) -> Self {
        Self {
            max_files: self.max_files.max(1),
            max_paths: self.max_paths.max(1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImpactAnalysis {
    pub affected_files: Vec<AffectedFile>,
    pub causal_changes: Vec<FileChange>,
    pub connecting_paths: Vec<ImpactPath>,
    pub limits: AnalysisLimits,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AffectedFile {
    pub path: String,
    pub impact: ImpactLevel,
    pub causal_change_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactLevel {
    Direct,
    Transitive,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImpactPath {
    pub change_index: usize,
    pub change_path: String,
    pub affected_path: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalysisLimits {
    pub max_files: usize,
    pub max_paths: usize,
    pub baseline_truncated: bool,
    pub current_truncated: bool,
    pub paths_truncated: bool,
    pub unknown: Vec<UnknownArea>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct UnknownArea {
    pub reason: UnknownReason,
    pub path: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    UnsupportedLanguage,
    UnresolvedImport,
    ParseFailure,
    FileLimit,
    PathLimit,
}

#[derive(Debug, Default)]
pub(super) struct AffectedAccumulator {
    pub(super) files: std::collections::BTreeMap<String, AffectedBuilder>,
    pub(super) paths: Vec<ImpactPath>,
    pub(super) limits: Vec<UnknownArea>,
}

#[derive(Debug)]
pub(super) struct AffectedBuilder {
    pub(super) impact: ImpactLevel,
    pub(super) changes: std::collections::BTreeSet<usize>,
}

impl AffectedAccumulator {
    pub(super) fn add_file(&mut self, path: &str, impact: ImpactLevel, change_index: usize) {
        let entry = self
            .files
            .entry(path.to_owned())
            .or_insert_with(|| AffectedBuilder {
                impact,
                changes: std::collections::BTreeSet::new(),
            });
        if impact == ImpactLevel::Unknown || entry.impact == ImpactLevel::Unknown {
            entry.impact = ImpactLevel::Unknown;
        } else if impact == ImpactLevel::Direct {
            entry.impact = ImpactLevel::Direct;
        }
        entry.changes.insert(change_index);
    }

    pub(super) fn add_unknown(&mut self, area: UnknownArea) {
        self.limits.push(area);
    }

    pub(super) fn finish(
        mut self,
        causal_changes: Vec<FileChange>,
        mut limits: AnalysisLimits,
    ) -> ImpactAnalysis {
        self.limits.sort();
        self.limits.dedup();
        limits.unknown.extend(self.limits);
        limits.unknown.sort();
        limits.unknown.dedup();
        let affected_files = self
            .files
            .into_iter()
            .map(|(path, builder)| AffectedFile {
                path,
                impact: builder.impact,
                causal_change_indices: builder.changes.into_iter().collect(),
            })
            .collect();
        let partial = !limits.unknown.is_empty()
            || limits.baseline_truncated
            || limits.current_truncated
            || limits.paths_truncated;
        ImpactAnalysis {
            affected_files,
            causal_changes,
            connecting_paths: self.paths,
            limits,
            partial,
        }
    }
}
