use anyhow::Result;

pub(super) fn checked_add(left: u64, right: u64, field: &str) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| anyhow::anyhow!("{field} overflow"))
}

pub(super) fn recent_average(tokens: &[u64], recent_turns: usize) -> Result<u64> {
    let start = tokens.len().saturating_sub(recent_turns);
    let mut previous = start
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .copied()
        .unwrap_or(0);
    let mut total = 0;
    for token in &tokens[start..] {
        total = checked_add(total, token.saturating_sub(previous), "recent token total")?;
        previous = *token;
    }
    Ok(total / u64::try_from(tokens[start..].len()).unwrap_or(1))
}
