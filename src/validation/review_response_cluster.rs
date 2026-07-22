use std::path::Path;

use serde::Deserialize;

use crate::paths::display_relative;

mod identity;
mod procedure;

use identity::{canonical, empty, nonempty_list};
const ORCHESTRATION_CLAUSES: &[&str] = &[
    "Before review-response edits, MUST create one root-cause cluster for each actionable defect class.",
    "Each cluster MUST name its stable defect class, violated invariant, structural boundary, related current threads, and representative positive and negative matrix.",
];
const GIT_WORKFLOW_CLAUSES: &[&str] = &[
    "Review-response handoffs MUST include a typed `ReviewClusterReceipt` for every actionable defect class before implementation edits begin.",
    "Each repaired cluster MUST record one structural repair and the removed case-specific behavior; quoted-input, phrase, or test-case exceptions are insufficient evidence.",
];
const PROOF_CLAUSES: &[&str] = &[
    "Review-response proof MUST exercise a bounded positive and negative matrix for every repaired root-cause cluster.",
    "A repeated same-class variant MUST NOT consume a new repair cycle unless its receipt names a distinct invariant or proves the prior structural repair incomplete.",
];
const SENTINEL_CLAUSES: &[&str] = &[
    "Sentinel MUST consolidate examples from the same defect class into one blocker with one structural repair strategy.",
    "A reopened resolved class MUST name a distinct violated invariant or prove the prior structural repair incomplete.",
];

pub(super) fn check_instruction_policy(path: &Path, text: &str, errors: &mut Vec<String>) {
    procedure::check(path, text, errors);
    let clauses = if path.ends_with("skills/codex-orchestration/SKILL.md") {
        Some(ORCHESTRATION_CLAUSES)
    } else if path.ends_with("skills/git-workflow/SKILL.md") {
        Some(GIT_WORKFLOW_CLAUSES)
    } else if path.ends_with("skills/proof-driven-completion/SKILL.md") {
        Some(PROOF_CLAUSES)
    } else if path.ends_with("agents/codexy-sentinel.toml") {
        Some(SENTINEL_CLAUSES)
    } else {
        None
    };
    let Some(clauses) = clauses else {
        return;
    };
    for clause in clauses {
        if !contains_clause(text, clause) {
            errors.push(format!(
                "{} root-cause review cluster contract failed: missing required clause",
                display_relative(path)
            ));
        }
    }
}

pub(super) fn diagnostics(receipt: &str) -> Vec<String> {
    let receipt = match serde_json::from_str::<ReviewClusterReceipt>(receipt) {
        Ok(receipt) => receipt,
        Err(error) => {
            return vec![format!(
                "root-cause review cluster receipt must be typed JSON: {error}"
            )];
        }
    };
    let mut errors = Vec::new();
    if receipt.clusters.is_empty() {
        errors.push("root-cause review cluster receipt must include a defect cluster".into());
    }
    let mut classes = std::collections::BTreeSet::new();
    let mut reopened = false;
    for cluster in &receipt.clusters {
        check_content(cluster, &mut errors);
        if !classes.insert(canonical(&cluster.defect_class)) {
            errors.push(format!(
                "root-cause cluster `{}` must consolidate same-class examples",
                cluster.defect_class
            ));
        }
        check_supplied_repair(cluster, &mut errors);
        if let Some(reopen) = &cluster.reopen {
            reopened = true;
            check_reopen(cluster, reopen, &mut errors);
        }
        check_state_transition(cluster, receipt.state, &mut errors);
    }
    if receipt.state == ReceiptState::Reopened && !reopened {
        errors.push("reopened review cluster receipt must prove a distinct invariant or incomplete structural repair".into());
    }
    errors
}

fn check_content(cluster: &DefectCluster, errors: &mut Vec<String>) {
    if empty(&cluster.defect_class)
        || empty(&cluster.violated_invariant)
        || empty(&cluster.structural_boundary)
        || !nonempty_list(&cluster.threads)
        || !nonempty_list(&cluster.matrix.positive)
        || !nonempty_list(&cluster.matrix.negative)
    {
        errors.push("root-cause cluster is missing typed class, invariant, boundary, thread, or matrix evidence".into());
    }
}

fn check_supplied_repair(cluster: &DefectCluster, errors: &mut Vec<String>) {
    match &cluster.repair {
        Some(Repair::Structural {
            boundary,
            strategy,
            removed_case_specific_behavior,
        }) if !empty(boundary) && !empty(strategy) && *removed_case_specific_behavior => {}
        Some(Repair::Structural { .. }) => errors.push(format!(
            "root-cause cluster `{}` structural repair must name its boundary, strategy, and removed case-specific behavior",
            cluster.defect_class
        )),
        Some(Repair::CaseException { quoted_input }) => errors.push(format!(
            "root-cause cluster `{}` requires one structural repair, not a case-specific exception for `{quoted_input}`",
            cluster.defect_class
        )),
        None => {}
    }
}

fn check_state_transition(cluster: &DefectCluster, state: ReceiptState, errors: &mut Vec<String>) {
    let repair_required = matches!(state, ReceiptState::Repaired | ReceiptState::Reopened);
    if repair_required && cluster.repair.is_none() {
        errors.push(format!(
            "{} root-cause cluster `{}` requires a structural repair",
            state.name(),
            cluster.defect_class
        ));
    }
    if state != ReceiptState::Reopened && cluster.reopen.is_some() {
        errors.push(format!(
            "{} root-cause cluster `{}` must not include reopened evidence",
            state.name(),
            cluster.defect_class
        ));
    }
}

fn check_reopen(cluster: &DefectCluster, reopen: &Reopen, errors: &mut Vec<String>) {
    match reopen {
        Reopen::DistinctInvariant { invariant }
            if !empty(invariant) && canonical(invariant) != canonical(&cluster.violated_invariant) => {}
        Reopen::StructuralRepairIncomplete { evidence } if !empty(evidence) => {}
        _ => errors.push(format!(
            "reopened cluster `{}` must name a distinct invariant or prove the prior structural repair incomplete",
            cluster.defect_class
        )),
    }
}

fn contains_clause(text: &str, clause: &str) -> bool {
    normalize(text).contains(&normalize(clause))
}

fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewClusterReceipt {
    state: ReceiptState,
    clusters: Vec<DefectCluster>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ReceiptState {
    Planned,
    Repaired,
    Reopened,
}

impl ReceiptState {
    const fn name(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Repaired => "repaired",
            Self::Reopened => "reopened",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DefectCluster {
    defect_class: String,
    violated_invariant: String,
    structural_boundary: String,
    threads: Vec<String>,
    matrix: RepresentativeMatrix,
    repair: Option<Repair>,
    reopen: Option<Reopen>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepresentativeMatrix {
    positive: Vec<String>,
    negative: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Repair {
    Structural {
        boundary: String,
        strategy: String,
        removed_case_specific_behavior: bool,
    },
    CaseException {
        quoted_input: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Reopen {
    DistinctInvariant { invariant: String },
    StructuralRepairIncomplete { evidence: String },
}
