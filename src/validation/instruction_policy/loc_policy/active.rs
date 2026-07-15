pub(super) fn governs_loc_exception(words: &[String], must: usize) -> bool {
    is_permission(words.get(must + 1).map(String::as_str))
        && words.get(must + 2..).is_some_and(|object| {
            matches!(
                object,
                [loc, exception, ..]
                    if loc == "loc" && matches!(exception.as_str(), "exception" | "exceptions")
            )
        })
}

pub(super) fn governs_overage(words: &[String], exceed: usize) -> bool {
    words[..exceed]
        .windows(2)
        .any(|pair| matches!(pair, [must, permission] if must == "must" && is_permission(Some(permission))))
}

fn is_permission(word: Option<&str>) -> bool {
    matches!(word, Some("allow" | "authorize" | "exempt" | "permit"))
}
