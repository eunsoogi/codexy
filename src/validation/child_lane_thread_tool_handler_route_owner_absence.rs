pub(super) fn has_route_owner_absence(tokens: &[&str]) -> bool {
    let tokens = strip_actor_article(tokens);
    if tokens.first().is_some_and(|first| {
        matches!(
            *first,
            "need"
                | "needed"
                | "use"
                | "used"
                | "actual"
                | "assigned"
                | "authorized"
                | "correct"
                | "current"
                | "expected"
                | "intended"
                | "primary"
                | "proper"
                | "real"
                | "responsible"
                | "right"
                | "same"
                | "valid"
        )
    }) {
        return has_route_owner_absence(&tokens[1..]);
    }
    if matches!(
        tokens,
        [
            "child",
            "thread",
            "tool",
            "handler" | "handlers",
            "are" | "is" | "was" | "were",
            "available" | "provided" | "registered",
            ..
        ]
    ) {
        return false;
    }
    matches!(
        tokens,
        ["child" | "owner", ..] | ["fallback", "route" | "path", ..] | ["route", "owner", ..]
    )
}

#[rustfmt::skip]
pub(super) fn strip_actor_article<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
    let mut tokens = tokens; while matches!(tokens.first().copied(), Some("a" | "an" | "any" | "from" | "member" | "of" | "one" | "single" | "the")) { tokens = &tokens[1..]; } tokens
}

#[cfg(test)]
mod tests {
    #[test]
    fn route_owner_dependency_is_one_way() {
        let route_value = include_str!("child_lane_thread_tool_handler_route_value.rs");
        let route_owner = include_str!("child_lane_thread_tool_handler_route_owner_absence.rs");
        let route_owner_production = route_owner
            .split_once("#[cfg(test)]")
            .map_or(route_owner, |(production, _)| production);

        assert!(route_value.contains("route_owner_absence"));
        assert!(!route_owner_production.contains("child_lane_thread_tool_handler_route_value"));
    }
}
