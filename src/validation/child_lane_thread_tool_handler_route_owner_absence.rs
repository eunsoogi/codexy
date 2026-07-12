use super::child_lane_thread_tool_handler_route_value::strip_actor_article;

pub(super) fn has_route_owner_absence(tokens: &[&str]) -> bool {
    let tokens = strip_actor_article(tokens);
    if let [first, rest @ ..] = tokens
        && matches!(
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
    {
        return has_route_owner_absence(rest);
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
