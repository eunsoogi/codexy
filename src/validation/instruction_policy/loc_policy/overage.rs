pub(super) fn is_negated(words: &[String], exceed: usize) -> bool {
    let local_start = words[..exceed]
        .iter()
        .rposition(|word| matches!(word.as_str(), "but" | "however" | "yet"))
        .map_or(0, |boundary| boundary + 1);
    let local = &words[local_start..];
    let exceed = exceed - local_start;
    let before = &local[..exceed];
    if before
        .iter()
        .rposition(|word| matches!(word.as_str(), "may" | "can"))
        .is_some_and(|modal| !before[modal + 1..].iter().any(|word| word == "not"))
    {
        return false;
    }
    let modal_negation = |part: &[String]| {
        part.windows(2).any(|pair| matches!(pair, [modal, not] if matches!(modal.as_str(), "must" | "may" | "can") && not == "not")) || part.iter().any(|word| matches!(word.as_str(), "cannot" | "never"))
    };
    modal_negation(before) || modal_negation(&local[exceed + 1..])
}
