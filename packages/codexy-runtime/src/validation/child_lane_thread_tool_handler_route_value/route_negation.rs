use super::super::child_lane_thread_tool_handler_route_owner_absence::{
    has_route_owner_absence, strip_actor_article,
};

pub(super) fn has_qualified_actor_negation(local: &str, at_boundary: bool) -> bool {
    const PROOF_VERBS: &str = "confirm|document|establish|prove|show|verify";
    let tokens = local.split_whitespace().collect::<Vec<_>>();
    let Some(negation_index) = tokens
        .iter()
        .rposition(|token| matches!(*token, "no" | "not") || token.ends_with("n't"))
    else {
        return false;
    };
    let mut after_not = &tokens[negation_index + 1..];
    while matches!(
        after_not.first().copied(),
        Some("actually" | "fully" | "really" | "truly")
    ) {
        after_not = &after_not[1..];
    }
    if after_not
        .first()
        .is_some_and(|token| PROOF_VERBS.split('|').any(|verb| *token == verb))
    {
        let subject = match after_not.get(1).copied() {
            Some("if" | "that" | "whether") => &after_not[2..],
            _ => &after_not[1..],
        };
        let actor = strip_actor_article(subject);
        return if actor.is_empty() {
            !at_boundary || !has_handler_tool_negation_subject(&tokens[..negation_index])
        } else {
            has_negated_actor_prefix(actor)
        };
    }
    has_route_owner_absence(after_not) || has_negated_actor_prefix(strip_actor_article(after_not))
}

fn has_handler_tool_negation_subject(tokens: &[&str]) -> bool {
    tokens
        .iter()
        .rev()
        .find(|token| {
            !matches!(
                **token,
                "actually" | "can" | "could" | "did" | "does" | "do" | "fully" | "really" | "truly"
            )
        })
        .is_some_and(|token| matches!(*token, "handler" | "tool"))
}

fn has_negated_actor_prefix(tokens: &[&str]) -> bool {
    const QUALIFIERS: &str = "actual|assigned|authorized|correct|current|expected|intended|primary|proper|real|responsible|right|same|valid";
    tokens.is_empty()
        || tokens
            .iter()
            .all(|token| QUALIFIERS.split('|').any(|qualifier| *token == qualifier))
}
