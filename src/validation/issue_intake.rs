use std::collections::BTreeSet;

use super::{
    conventional_commit,
    issue_intake_receipt::{
        ApprovalDecision, Candidate, Classification, DuplicateConclusion, DuplicateSearch,
        EvidenceDecision, IssueState, MatchKind, NecessityDecision, OwnershipDecision,
    },
    markdown,
};

const REQUIRED_SECTIONS: [&str; 4] = [
    "## Problem",
    "## Scope",
    "## Acceptance Criteria",
    "## Verification",
];

pub(super) fn check(text: &str) -> Vec<String> {
    let candidate = match serde_json::from_str::<Candidate>(text) {
        Ok(candidate) => candidate,
        Err(error) => return vec![format!("issue-intake receipt is invalid JSON: {error}")],
    };
    let mut errors = Vec::new();
    check_decisions(&candidate, &mut errors);
    check_evidence(&candidate, &mut errors);
    check_duplicates(&candidate.duplicate_search, &mut errors);
    check_issue_content(&candidate, &mut errors);
    check_metadata(&candidate, &mut errors);
    errors
}

fn check_decisions(candidate: &Candidate, errors: &mut Vec<String>) {
    if candidate.parent_approval.decision != ApprovalDecision::Approved
        || !is_task_id(&candidate.parent_approval.source_task_id)
    {
        errors.push("issue-intake requires explicit parent approval from a source task".into());
    }
    if !matches!(candidate.classification, Classification::IssueSizedDefect) {
        errors.push("unsupported synthetic and same-class observations are handoff-only".into());
    }
    if candidate.reproduction.decision != EvidenceDecision::Supported {
        errors.push("unsupported reproduction is handoff-only".into());
    }
    if candidate.ownership.decision != OwnershipDecision::CannotOwn {
        errors.push("candidate already owned by an issue or PR is handoff-only".into());
    }
    if candidate.necessity.decision == NecessityDecision::NoChange {
        errors.push("no-change candidate is handoff-only".into());
    }
}

fn check_evidence(candidate: &Candidate, errors: &mut Vec<String>) {
    let reproduction = &candidate.reproduction;
    if !substantive(&reproduction.surface)
        || reproduction.steps.is_empty()
        || reproduction.steps.iter().any(|step| !substantive(step))
        || !substantive(&reproduction.observed)
    {
        errors.push("issue-intake requires substantive supported-surface reproduction".into());
    }
    let owner = &candidate.ownership.existing_owner;
    if owner.number == 0 || !substantive(&candidate.ownership.rationale) {
        errors.push("issue-intake requires substantive ownership rationale".into());
    }
    if !substantive(&candidate.necessity.rationale) {
        errors.push("issue-intake requires substantive thin-harness necessity".into());
    }
}

fn check_duplicates(search: &DuplicateSearch, errors: &mut Vec<String>) {
    let states = search.states.iter().copied().collect::<BTreeSet<_>>();
    if states != BTreeSet::from([IssueState::Open, IssueState::Closed]) {
        errors.push("issue-intake requires open and closed duplicate-search states".into());
    }
    if search.search_terms.is_empty() || search.search_terms.iter().any(|term| !substantive(term)) {
        errors.push("issue-intake requires substantive duplicate-search terms".into());
    }
    let exact = search
        .results
        .iter()
        .filter(|result| result.match_kind == MatchKind::Exact)
        .map(|result| result.issue)
        .collect::<BTreeSet<_>>();
    if search.results.iter().any(|result| result.issue == 0) {
        errors.push("issue-intake duplicate results require canonical issue numbers".into());
    }
    match search.conclusion {
        DuplicateConclusion::NoDuplicate if !exact.is_empty() => {
            errors.push("issue-intake exact duplicate result contradicts no_duplicate".into());
        }
        DuplicateConclusion::Duplicate { canonical_issue } => {
            if !exact.contains(&canonical_issue) {
                errors
                    .push("issue-intake duplicate conclusion must identify an exact result".into());
            }
            errors.push(format!(
                "issue-intake candidate duplicates canonical issue #{canonical_issue}"
            ));
        }
        DuplicateConclusion::NoDuplicate => {}
    }
}

fn check_issue_content(candidate: &Candidate, errors: &mut Vec<String>) {
    if !conventional_commit::check_issue_title(&candidate.title).is_empty() {
        errors.push("issue-intake title is not a validated descriptive issue title".into());
    }
    for section in REQUIRED_SECTIONS {
        if !markdown::has_heading(&candidate.body, section) {
            errors.push(format!("issue-intake body is missing {section}"));
        }
    }
}

fn check_metadata(candidate: &Candidate, errors: &mut Vec<String>) {
    let repository_labels = candidate
        .repository_labels
        .iter()
        .map(|label| label.trim())
        .collect::<BTreeSet<_>>();
    if repository_labels.is_empty()
        || repository_labels.contains("")
        || candidate.labels.is_empty()
        || candidate
            .labels
            .iter()
            .map(|label| label.trim())
            .any(|label| label.is_empty() || !repository_labels.contains(label))
    {
        errors.push("issue-intake labels must be repository-valid".into());
    }
    let repository_milestones = candidate
        .repository_milestones
        .iter()
        .map(|milestone| milestone.trim())
        .collect::<BTreeSet<_>>();
    let milestone = candidate.milestone.trim();
    if repository_milestones.is_empty() || repository_milestones.contains("") {
        errors.push("issue-intake requires a non-empty repository milestone taxonomy".into());
    } else if milestone.is_empty() || !repository_milestones.contains(milestone) {
        errors.push("issue-intake milestone must be repository-valid".into());
    }
    let repository_assignees = candidate
        .repository_assignees
        .iter()
        .map(|assignee| assignee.trim())
        .collect::<BTreeSet<_>>();
    let assignee = candidate.assignee.trim();
    if repository_assignees.is_empty() || repository_assignees.contains("") {
        errors.push("issue-intake requires a non-empty repository assignee taxonomy".into());
    } else if assignee.is_empty() || !repository_assignees.contains(assignee) {
        errors.push("issue-intake assignee must be repository-valid".into());
    }
}

fn substantive(value: &str) -> bool {
    value
        .split_whitespace()
        .filter_map(|token| {
            let normalized = token
                .chars()
                .filter(|character| character.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>();
            (normalized.chars().count() >= 2).then_some(normalized)
        })
        .collect::<BTreeSet<_>>()
        .len()
        >= 2
}

fn is_task_id(value: &str) -> bool {
    value.len() == 36
        && value
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(index, item)| match index {
                8 | 13 | 18 | 23 => *item == b'-',
                _ => item.is_ascii_hexdigit(),
            })
}
