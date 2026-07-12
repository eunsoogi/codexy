pub(super) fn luna_policy_clauses(segment: &str) -> impl Iterator<Item = &str> {
    segment
        .split(" but ")
        .flat_map(|clause| clause.split(" and "))
}

pub(super) fn has_luna_default_assignment(segment: &str) -> bool {
    [
        "be the blanket default",
        "be a blanket default",
        "use luna as the blanket default",
        "use luna as a blanket default",
        "make luna the blanket default",
        "make luna a blanket default",
    ]
    .iter()
    .any(|assignment| segment.contains(assignment))
}

pub(super) fn luna_blanket_default_is_negated(segment: &str) -> bool {
    [
        "not be the blanket default",
        "not be a blanket default",
        "not the blanket default",
        "not a blanket default",
        "not as the blanket default",
        "not as a blanket default",
        "never be the blanket default",
        "never be a blanket default",
    ]
    .iter()
    .filter_map(|negation| segment.find(negation))
    .min()
    .is_some_and(|index| !has_luna_default_assignment(&segment[..index]))
}
