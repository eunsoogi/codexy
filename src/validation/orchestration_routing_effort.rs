pub(super) fn assigned_reasoning_efforts(segment: &str) -> Vec<&str> {
    ["reasoning_effort", "reasoning-effort"]
        .iter()
        .flat_map(|field| {
            segment.match_indices(field).filter_map(|(index, _)| {
                let mut values = segment[index + field.len()..]
                    .trim_start_matches(|character: char| {
                        character.is_ascii_whitespace()
                            || matches!(character, ':' | '=' | '`' | '\"' | '\'')
                    })
                    .split(|character: char| {
                        character.is_ascii_whitespace()
                            || matches!(character, ',' | ';' | '.' | '`' | '\"' | '\'')
                    })
                    .filter(|value| !value.is_empty());
                let value = values.next()?;
                let value = (value == "to")
                    .then(|| values.next())
                    .flatten()
                    .unwrap_or(value);
                matches!(value, "low" | "medium" | "high" | "xhigh" | "max" | "ultra")
                    .then_some(value)
            })
        })
        .collect()
}
