pub(super) fn governs_loc_exception(words: &[String], approval: usize) -> bool {
    let passive = approval >= 3
        && matches!(
            &words[approval - 3..approval],
            [loc, exception, verb]
                if loc == "loc"
                    && matches!(exception.as_str(), "exception" | "exceptions")
                    && matches!(verb.as_str(), "is" | "are")
        );
    let object = &words[approval + 1..];
    let object = if object.first().is_some_and(|word| word == "the") {
        &object[1..]
    } else {
        object
    };
    let active = matches!(
        object,
        [loc, exception, ..]
            if loc == "loc" && matches!(exception.as_str(), "exception" | "exceptions")
    );
    passive || active || requires_approval(words, approval)
}

fn requires_approval(words: &[String], approval: usize) -> bool {
    if words[approval] != "approval" {
        return false;
    }
    words[..approval]
        .windows(2)
        .rposition(|pair| matches!(pair, [loc, exception] if loc == "loc" && matches!(exception.as_str(), "exception" | "exceptions")))
        .is_some_and(|exception| {
            let requirement = &words[exception + 2..approval];
            requirement
                .iter()
                .any(|word| matches!(word.as_str(), "require" | "required" | "requires"))
                && !requirement
                    .iter()
                    .any(|word| matches!(word.as_str(), "no" | "not" | "never"))
        })
}
