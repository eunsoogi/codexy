pub(crate) fn normalize_instruction_text(text: &str) -> String {
    let mut in_allowed_actions = false;
    text.lines()
        .map(str::trim)
        .map(|line| {
            if line.is_empty() {
                in_allowed_actions = false;
                return String::new();
            }
            if is_action_heading(line) {
                in_allowed_actions = is_allowed_actions_heading(line);
                return line.to_owned();
            }
            if line.starts_with('#') {
                in_allowed_actions = false;
                return line.to_owned();
            }
            let item = item_text(line);
            if in_allowed_actions && item.is_some() {
                allowed_action_item(item.unwrap_or(line))
            } else {
                item.unwrap_or(line).to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_allowed_actions_heading(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("allowed actions") || lower.starts_with("permitted actions")
}

fn is_action_heading(line: &str) -> bool {
    line.to_ascii_lowercase().ends_with("actions:")
}

fn item_text(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| {
            line.split_once(". ")
                .filter(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
                .map(|(_, remainder)| remainder)
        })
}

fn allowed_action_item(item: &str) -> String {
    format!("Allowed actions: {item}")
}
