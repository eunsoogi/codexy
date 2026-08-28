#![allow(dead_code)]

pub(super) fn is_adjunct_preposition(word: &str) -> bool {
    matches!(
        word,
        "at" | "by" | "for" | "from" | "in" | "on" | "under" | "with" | "without"
    )
}

pub(super) fn is_condition_phrase_boundary(word: &str) -> bool {
    matches!(word, "a" | "an" | "the" | "and" | "or" | "but")
        || is_preposition_or_subordinator(word)
        || is_auxiliary(word)
}

fn is_preposition_or_subordinator(word: &str) -> bool {
    word.ends_with("ing")
        || matches!(
            word,
            "aboard"
                | "about"
                | "above"
                | "across"
                | "after"
                | "against"
                | "along"
                | "among"
                | "amid"
                | "amidst"
                | "around"
                | "as"
                | "at"
                | "before"
                | "behind"
                | "below"
                | "beneath"
                | "beside"
                | "besides"
                | "between"
                | "beyond"
                | "by"
                | "concerning"
                | "considering"
                | "despite"
                | "down"
                | "during"
                | "except"
                | "excluding"
                | "following"
                | "for"
                | "from"
                | "given"
                | "in"
                | "including"
                | "inside"
                | "into"
                | "like"
                | "near"
                | "of"
                | "off"
                | "on"
                | "onto"
                | "opposite"
                | "outside"
                | "over"
                | "past"
                | "per"
                | "regarding"
                | "round"
                | "since"
                | "than"
                | "through"
                | "throughout"
                | "till"
                | "to"
                | "toward"
                | "towards"
                | "under"
                | "underneath"
                | "until"
                | "up"
                | "upon"
                | "via"
                | "versus"
                | "vs"
                | "with"
                | "within"
                | "without"
                | "although"
                | "because"
                | "if"
                | "lest"
                | "once"
                | "provided"
                | "though"
                | "unless"
                | "when"
                | "whenever"
                | "whereas"
                | "wherever"
                | "while"
                | "whether"
        )
}

fn is_auxiliary(word: &str) -> bool {
    matches!(
        word,
        "am" | "are"
            | "be"
            | "been"
            | "being"
            | "can"
            | "could"
            | "did"
            | "do"
            | "does"
            | "had"
            | "has"
            | "have"
            | "is"
            | "may"
            | "might"
            | "must"
            | "shall"
            | "should"
            | "was"
            | "were"
            | "will"
            | "would"
    )
}
