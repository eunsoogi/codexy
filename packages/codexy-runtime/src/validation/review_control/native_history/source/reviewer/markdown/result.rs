pub(super) fn parse(value: &str) -> Option<(String, usize)> {
    let leading = value.len() - value.trim_start().len();
    let mut value = &value[leading..];
    let mut start = leading;
    const HEADING_PREFIX: &str = "## Blocking findings — ";
    if value
        .get(..HEADING_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(HEADING_PREFIX))
    {
        start += HEADING_PREFIX.len();
        value = &value[HEADING_PREFIX.len()..];
    }
    let value_leading = value.len() - value.trim_start().len();
    let value = value.trim();
    start += value_leading;
    let mut inner = value;
    for (open, close) in [
        ("`", "`"),
        ("**", "**"),
        ("__", "__"),
        ("*", "*"),
        ("_", "_"),
    ] {
        if inner.len() >= open.len() + close.len()
            && inner.starts_with(open)
            && inner.ends_with(close)
        {
            start += open.len();
            inner = &inner[open.len()..inner.len() - close.len()];
            break;
        }
    }
    let inner_leading = inner.len() - inner.trim_start().len();
    let inner = inner.trim();
    start += inner_leading;
    let result = inner.to_ascii_uppercase();
    ["PASS", "BLOCK", "UNOBSERVABLE"]
        .contains(&result.as_str())
        .then_some((result, start))
}
