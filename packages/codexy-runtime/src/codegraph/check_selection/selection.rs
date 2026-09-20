use std::collections::{BTreeMap, BTreeSet};

use super::super::change_impact::{ImpactAnalysis, ImpactLevel};
use super::PathEvidence;
use super::classify::{impact_label, mapping_matches};
use super::concerns::{
    add_area_concern, add_global_concern, add_path_concern, area_label, finish_broader,
    finish_manual, mapping_kind_label, owner_label,
};
use super::model::{
    BroaderReason, ChangeArea, CheckDefinition, CheckMapping, CheckMappings, CheckRecommendation,
    DependencyState, GapReason, ManualJudgment, ManualReason, SelectionGap, SelectionOptions,
};

pub(super) struct SelectionOutput {
    pub(super) recommendations: Vec<CheckRecommendation>,
    pub(super) gaps: Vec<SelectionGap>,
    pub(super) broader_verification: Vec<super::model::BroaderVerification>,
    pub(super) manual_judgment: Vec<ManualJudgment>,
    pub(super) partial: bool,
}

pub(super) fn select(
    paths: &BTreeMap<String, PathEvidence>,
    impact: &ImpactAnalysis,
    mappings: &CheckMappings,
    options: &SelectionOptions,
) -> SelectionOutput {
    let mut state = SelectionState::new(mappings);
    for (path, evidence) in paths {
        state.add_path(path, evidence, mappings);
    }

    if impact.partial {
        add_global_concern(&mut state.broader, BroaderReason::PartialImpact);
        add_global_concern(&mut state.manual, ManualReason::PartialAnalysis);
    }
    if !impact.limits.unknown.is_empty() {
        add_global_concern(&mut state.manual, ManualReason::PartialAnalysis);
    }
    let dependency_unconfirmed = !paths.is_empty()
        && matches!(
            options.dependency_state,
            DependencyState::Unconfirmed { .. }
        );
    if dependency_unconfirmed {
        add_global_concern(&mut state.broader, BroaderReason::UnconfirmedDependencies);
        add_global_concern(&mut state.manual, ManualReason::UnconfirmedDependencies);
    }

    let partial = impact.partial || !state.gaps.is_empty() || dependency_unconfirmed;
    SelectionOutput {
        recommendations: state
            .recommendations
            .into_values()
            .map(RecommendationBuilder::finish)
            .collect(),
        gaps: state.gaps.into_iter().collect(),
        broader_verification: finish_broader(state.broader),
        manual_judgment: finish_manual(state.manual),
        partial,
    }
}

struct SelectionState {
    definitions: BTreeMap<String, CheckDefinition>,
    recommendations: BTreeMap<String, RecommendationBuilder>,
    gaps: BTreeSet<SelectionGap>,
    broader: BTreeMap<BroaderReason, BTreeSet<String>>,
    manual: BTreeMap<ManualReason, BTreeSet<String>>,
}

impl SelectionState {
    fn new(mappings: &CheckMappings) -> Self {
        Self {
            definitions: mappings
                .checks
                .iter()
                .cloned()
                .map(|check| (check.id.clone(), check))
                .collect(),
            recommendations: BTreeMap::new(),
            gaps: BTreeSet::new(),
            broader: BTreeMap::new(),
            manual: BTreeMap::new(),
        }
    }

    fn add_path(&mut self, path: &str, evidence: &PathEvidence, mappings: &CheckMappings) {
        add_area_concern(&mut self.broader, &evidence.area, path);
        if evidence.impact == ImpactLevel::Unknown {
            add_path_concern(&mut self.broader, BroaderReason::UnknownImpact, path);
            add_path_concern(&mut self.manual, ManualReason::UnknownImpact, path);
            self.gaps.insert(SelectionGap {
                path: Some(path.to_owned()),
                area: Some(evidence.area.clone()),
                mapping_pattern: None,
                reason: GapReason::UnknownImpact,
                detail: "impact analysis could not establish a complete dependency result".into(),
            });
        }

        let matching = mappings
            .mappings
            .iter()
            .filter(|mapping| mapping_matches(&mapping.pattern, path));
        let mut matched = false;
        for mapping in matching {
            matched = true;
            self.add_mapping(path, evidence, mapping);
        }
        if !matched {
            self.gaps.insert(SelectionGap {
                path: Some(path.to_owned()),
                area: Some(evidence.area.clone()),
                mapping_pattern: None,
                reason: GapReason::MissingMapping,
                detail: "no explicit check mapping matched this path".into(),
            });
            add_path_concern(&mut self.manual, ManualReason::MissingMapping, path);
        }
    }

    fn add_mapping(&mut self, path: &str, evidence: &PathEvidence, mapping: &CheckMapping) {
        if mapping.check_ids.is_empty() {
            self.gaps.insert(SelectionGap {
                path: Some(path.to_owned()),
                area: Some(evidence.area.clone()),
                mapping_pattern: Some(mapping.pattern.clone()),
                reason: GapReason::EmptyMapping,
                detail: "the matching mapping names no checks".into(),
            });
            add_path_concern(&mut self.manual, ManualReason::MissingCheck, path);
            return;
        }
        for check_id in &mapping.check_ids {
            let Some(check) = self.definitions.get(check_id) else {
                self.gaps.insert(SelectionGap {
                    path: Some(path.to_owned()),
                    area: Some(evidence.area.clone()),
                    mapping_pattern: Some(mapping.pattern.clone()),
                    reason: GapReason::MissingCheck,
                    detail: format!("mapping references unknown check `{check_id}`"),
                });
                add_path_concern(&mut self.manual, ManualReason::MissingCheck, path);
                continue;
            };
            if check.command.is_empty() {
                self.gaps.insert(SelectionGap {
                    path: Some(path.to_owned()),
                    area: Some(evidence.area.clone()),
                    mapping_pattern: Some(mapping.pattern.clone()),
                    reason: GapReason::MissingCommand,
                    detail: format!("check `{check_id}` has no command text"),
                });
                add_path_concern(&mut self.manual, ManualReason::MissingCheck, path);
                continue;
            }
            let entry = self
                .recommendations
                .entry(check.id.clone())
                .or_insert_with(|| RecommendationBuilder::new(check));
            entry.paths.insert(path.to_owned());
            entry.areas.insert(evidence.area.clone());
            entry.reasons.insert(format!(
                "{} {} mapping `{}` matched a {} {} change",
                owner_label(mapping),
                mapping_kind_label(mapping),
                mapping.pattern,
                area_label(&evidence.area),
                impact_label(evidence.impact),
            ));
            if !mapping.reason.is_empty() {
                entry.reasons.insert(mapping.reason.clone());
            }
        }
    }
}

#[derive(Debug)]
struct RecommendationBuilder {
    check: CheckDefinition,
    paths: BTreeSet<String>,
    areas: BTreeSet<ChangeArea>,
    reasons: BTreeSet<String>,
}

impl RecommendationBuilder {
    fn new(check: &CheckDefinition) -> Self {
        Self {
            check: check.clone(),
            paths: BTreeSet::new(),
            areas: BTreeSet::new(),
            reasons: BTreeSet::new(),
        }
    }

    fn finish(self) -> CheckRecommendation {
        CheckRecommendation {
            id: self.check.id,
            command: self.check.command,
            description: self.check.description,
            paths: self.paths.into_iter().collect(),
            areas: self.areas.into_iter().collect(),
            reasons: self.reasons.into_iter().collect(),
        }
    }
}
