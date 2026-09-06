use super::{Setting, clean, operative_lines};

pub(super) fn reviewer_setting(raw: &str) -> Result<Option<Setting>, String> {
    let mut matches = Vec::new();
    for (start, end, line) in operative_lines(raw) {
        let lower = line.to_ascii_lowercase();
        let Some(index) = lower.find("reviewer setting:") else {
            continue;
        };
        let value = line[index + "reviewer setting:".len()..].trim();
        let Some((model, effort)) = value.split_once('/') else {
            return Err("markdown reviewer setting label is invalid".into());
        };
        let model = clean(model);
        let effort = clean(effort);
        if model.is_empty() || effort.is_empty() {
            return Err("markdown reviewer setting label is invalid".into());
        }
        matches.push(Setting {
            model,
            effort,
            start,
            end,
        });
    }
    if matches
        .windows(2)
        .any(|pair| pair[0].model != pair[1].model || pair[0].effort != pair[1].effort)
    {
        return Err("markdown reviewer setting is contradictory".into());
    }
    Ok(matches.into_iter().next())
}
