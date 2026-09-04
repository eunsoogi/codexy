pub(super) fn binding<'a>(lines: &'a [&'a str]) -> Result<&'a str, &'static str> {
    let assignment = unique_value(lines, "assignment objective: ")
        .ok_or("clear delegated assignment requires one assignment objective")?;
    unique_value(lines, "success criteria: ")
        .ok_or("clear delegated assignment requires one success criteria record")?;
    let authorized = unique_value(lines, "authorized goal objective: ")
        .ok_or("clear delegated assignment requires one authorized goal objective")?;
    if assignment != authorized {
        return Err("authorized goal objective must exactly match the assignment objective");
    }
    Ok(authorized)
}

fn unique_value<'a>(lines: &'a [&'a str], prefix: &str) -> Option<&'a str> {
    let mut values = lines
        .iter()
        .filter_map(|line| strip_prefix_ascii_case(line.trim(), prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn strip_prefix_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}
