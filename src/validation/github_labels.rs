mod evidence;

use serde_json::Value;

use super::handoff_claims::has_negative_label_value;
use evidence::{
    is_codexy_lane, is_open_pr, is_stacked_pr, issue_label_errors, issue_nodes, label_names,
    repository_label_taxonomy, stacked_issue_evidence,
};

pub(super) fn check_completion_handoff(handoff: &str, pr_state: &str) -> Vec<String> {
    if !(claims_pr_readiness(handoff) || claims_completion(handoff)) {
        return Vec::new();
    }
    let pr_state = match serde_json::from_str::<Value>(pr_state) {
        Ok(value) => value,
        Err(error) => return vec![format!("GitHub label PR state JSON error: {error}")],
    };
    if !is_open_pr(&pr_state) || !is_codexy_lane(&pr_state) {
        return Vec::new();
    }
    if has_label_consideration_evidence(handoff) {
        match repository_label_taxonomy(&pr_state) {
            Some(labels) if labels.is_empty() => return Vec::new(),
            None => return vec!["GitHub label evidence missing repositoryLabels taxonomy".into()],
            _ => {}
        }
    }
    let mut errors = Vec::new();
    if label_names(pr_state.get("labels")).is_empty() {
        errors.push("PR labels missing label application evidence".into());
    }
    if is_stacked_pr(&pr_state) {
        match stacked_issue_evidence(&pr_state) {
            Some(issues) => errors.extend(issue_label_errors(issues)),
            None => errors.push(
                "GitHub label evidence missing stacked linkedIssueReferences with issue labels"
                    .into(),
            ),
        }
    } else {
        let closing_issues = issue_nodes(pr_state.get("closingIssuesReferences"));
        if closing_issues.is_empty() {
            errors.push(
                "GitHub label evidence missing closingIssuesReferences with issue labels".into(),
            );
        } else {
            errors.extend(issue_label_errors(closing_issues));
        }
    }
    errors
}

fn has_label_consideration_evidence(handoff: &str) -> bool {
    handoff.lines().any(|line| {
        let line = line.to_ascii_lowercase();
        "labels considered|label consideration"
            .split('|')
            .any(|phrase| line.contains(phrase))
            && "no matching|no-match|no applicable|not applicable|not-applicable"
                .split('|')
                .any(|phrase| line.contains(phrase))
            && !"missing|empty|absent|not applied|without|no labels"
                .split('|')
                .any(|phrase| line.contains(phrase))
    })
}

fn claims_pr_readiness(handoff: &str) -> bool {
    let text = handoff.to_ascii_lowercase();
    [
        "merge-ready",
        "merge-readiness",
        "merge readiness",
        "merge ready",
        "ready to merge",
        "ready for merge",
        "pr-ready",
        "pr-readiness",
        "pr readiness",
        "pr ready",
        "pr is ready",
        "pull request is ready",
    ]
    .into_iter()
    .any(|phrase| has_unnegated_readiness_phrase(&text, phrase, 24))
}

fn claims_completion(handoff: &str) -> bool {
    let mut text = handoff.to_ascii_lowercase();
    if has_not_complete_until_merge(&text) {
        text = text.replace("verification completed.", "verification evidence.");
        text = text.replace("verification completed:", "verification evidence:");
        for phrase in [
            "successfully completed",
            "completed successfully",
            "completed",
            "finished",
            "finalized",
        ] {
            text = text.replace(&format!("verification {phrase};"), "verification evidence;");
        }
    }
    ["completed", "finished", "finalized", "all set"]
        .iter()
        .any(|phrase| has_unnegated_phrase(&text, phrase, 16))
        || ["done", "complete", "completes", "finish", "finalize"]
            .iter()
            .any(|word| has_unnegated_phrase(&text, word, 16))
}

fn has_not_complete_until_merge(text: &str) -> bool {
    "not complete until merge|not currently complete until merge|isn't complete until merge|isn't currently complete until merge|aren't complete until merge|aren't currently complete until merge"
        .split('|')
        .any(|phrase| has_unnegated_phrase(text, phrase, 16))
}

fn has_unnegated_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
            && !has_nearby_negation(
                &text[char_window_start(text, absolute_index, negation_window)..absolute_index],
            )
        {
            return true;
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_unnegated_readiness_phrase(text: &str, phrase: &str, negation_window: usize) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let absolute_index = offset + index;
        let after_index = absolute_index + phrase.len();
        if is_boundary(text[..absolute_index].chars().next_back())
            && is_boundary(text[after_index..].chars().next())
            && !has_nearby_negation(
                &text[char_window_start(text, absolute_index, negation_window)..absolute_index],
            )
            && !has_negative_label_value(&text[after_index..])
        {
            return true;
        }
        offset = after_index;
        rest = &text[offset..];
    }
    false
}

fn has_nearby_negation(prefix: &str) -> bool {
    "no|not|not yet|not currently|without|isn't|is not"
        .split('|')
        .any(|phrase| prefix.trim_end().ends_with(phrase))
}

fn char_window_start(text: &str, end: usize, window: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .nth(window)
        .map_or(0, |(index, _)| index)
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| !character.is_ascii_alphanumeric())
}
