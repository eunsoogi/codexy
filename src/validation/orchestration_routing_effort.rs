pub(super) fn assigned_reasoning_efforts(segment: &str) -> Vec<&str> {
    ["reasoning_effort", "reasoning-effort"]
        .iter()
        .flat_map(|field| {
            segment.match_indices(field).filter_map(|(index, _)| {
                let value = segment[index + field.len()..]
                    .trim_start_matches(|character: char| {
                        character.is_ascii_whitespace()
                            || matches!(character, ':' | '=' | '`' | '\"' | '\'')
                    })
                    .split(|character: char| {
                        character.is_ascii_whitespace()
                            || matches!(character, ',' | ';' | '.' | '`' | '\"' | '\'')
                    })
                    .next()?;
                matches!(value, "low" | "medium" | "high" | "xhigh" | "max" | "ultra")
                    .then_some(value)
            })
        })
        .collect()
}
