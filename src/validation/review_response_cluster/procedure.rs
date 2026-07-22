use std::path::Path;

use crate::paths::display_relative;

const REFERENCE_PATH: &str = "skills/git-workflow/references/review-response-clusters.md";
const HEADING: &str = "## Required Procedure";

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !path.ends_with(REFERENCE_PATH) {
        return;
    }
    let mut in_procedure = false;
    let mut saw_step = false;
    for line in text.lines() {
        let line = line.trim();
        if line == HEADING {
            in_procedure = true;
            continue;
        }
        if in_procedure && line.starts_with("## ") {
            break;
        }
        if in_procedure && is_numbered_step(line) {
            saw_step = true;
            if !line.split_whitespace().any(|word| word == "MUST") {
                errors.push(format!(
                    "{} review procedure step must use MUST or MUST NOT",
                    display_relative(path)
                ));
            }
        }
    }
    if !saw_step {
        errors.push(format!(
            "{} review procedure must include numbered MUST/MUST NOT steps",
            display_relative(path)
        ));
    }
}

fn is_numbered_step(line: &str) -> bool {
    let Some((prefix, content)) = line.split_once(". ") else {
        return false;
    };
    !prefix.is_empty() && prefix.bytes().all(|byte| byte.is_ascii_digit()) && !content.is_empty()
}
