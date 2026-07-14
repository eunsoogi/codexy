use std::path::Path;

use crate::paths::display_relative;

const ORCHESTRATION_CLAUSES: &[&str] = &[
    "search the callable tool surface for `automation_update`",
    "register a thread-targeted `kind=heartbeat`",
    "creation MUST use `destination=\"thread\"`",
    "automation id, target thread, bounded schedule, stable observed-state identity, eligible material events, and terminal delete/disable action",
    "prompt MUST suppress unchanged observations and MUST wake the owner only for a material gate change or an explicit user/parent message",
    "MUST end its active goal and plan before waiting",
    "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
    "MUST consume the event in the same turn",
    "MUST delete or disable the heartbeat when no further observation is required",
    "MUST record the exact discovery/exposure evidence and use a bounded fallback",
    "without fabricating a monitor identity",
    "MUST mark automation id, schedule, and lifecycle as not-created",
    "MUST NOT fold a live packaged Sentinel into heartbeat observation",
    "read-only, event-driven, and subject to its no-poll/no-message boundary",
];

const TOKEN_CLAUSES: &[&str] = &[
    "heartbeat automation id",
    "bounded schedule, state fingerprint, material-event set, and delete/disable state",
    "MUST suppress unchanged observations",
    "material gate change or an explicit user/parent message",
    "active goal and plan MUST end before runtime-owned waiting",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
];

const EXTERNAL_GATE_CLAUSES: &[&str] = &[
    "MUST follow `references/runtime-heartbeats.md`",
    "parent or child MUST NOT retain an active goal or plan during an external-gate wait",
    "child external-gate wait MUST end its active goal and plan before waiting",
    "qualifying event starts a fresh short-lived execution goal",
    "heartbeat automation route MUST NOT require a persistent exec/session id or same-process resume",
];

const TEMPLATE_CLAUSES: &[&str] = &[
    "callable discovery/exposure evidence:",
    "heartbeat automation id:",
    "target thread:",
    "bounded schedule:",
    "state fingerprint:",
    "eligible material events:",
    "unchanged observations suppressed:",
    "terminal delete/disable action:",
];

const TRANSITION_CLAUSES: &[&str] = &[
    "heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "MUST NOT require a persistent exec/session identifier or same-process resume",
    "persistent exec/session identifier, a scheduled next-observation deadline, the last observed state fingerprint or event identity, and same-process resume",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    let (requirement, clauses) = if path.ends_with("skills/codex-orchestration/SKILL.md") {
        (
            "orchestration skill must preserve the runtime heartbeat external-gate policy",
            EXTERNAL_GATE_CLAUSES,
        )
    } else if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md") {
        (
            "runtime heartbeat contract must preserve its lifecycle policy",
            ORCHESTRATION_CLAUSES,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/SKILL.md") {
        (
            "token-efficient skill must preserve the runtime heartbeat contract",
            TOKEN_CLAUSES,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/templates/delta-poll.md") {
        (
            "runtime heartbeat delta template must preserve lifecycle slots",
            TEMPLATE_CLAUSES,
        )
    } else if path.ends_with("skills/codex-orchestration/references/goal-transition-reporting.md") {
        (
            "goal transition contract must distinguish heartbeat and process monitor identities",
            TRANSITION_CLAUSES,
        )
    } else {
        return;
    };
    let normalized = normalized_policy_text(text);
    for clause in clauses {
        let clause = normalized_policy_text(clause);
        if !has_unweakened_clause(&normalized, &clause) {
            errors.push(format!(
                "{} {requirement}: missing `{clause}`",
                display_relative(path)
            ));
        }
    }
    if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md")
        && normalized.contains("may fold a live packaged sentinel into heartbeat observation")
    {
        errors.push(format!(
            "{} runtime heartbeat contract must not permit Sentinel heartbeat observation",
            display_relative(path)
        ));
    }
}

fn has_unweakened_clause(text: &str, clause: &str) -> bool {
    text.match_indices(clause).any(|(index, _)| {
        let before = &text[..index];
        let after = text[index + clause.len()..]
            .trim_start_matches([',', ':', ';', '-', '—'])
            .trim_start();
        before.rfind("<markdown-heading>") <= before.rfind("</markdown-heading>")
            && !before
                .rsplit_once("</markdown-heading>")
                .map_or(before, |(_, current_section)| current_section)
                .rsplit(['.', ';'])
                .next()
                .is_some_and(|prefix| {
                    [
                        "historical example",
                        "false that",
                        "not required",
                        "no longer required",
                    ]
                    .iter()
                    .any(|marker| prefix.contains(marker))
                })
            && !["unless ", "except ", "only if ", "may ", "is not required"]
                .iter()
                .any(|marker| after.starts_with(marker))
    })
}
fn normalized_policy_text(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut historical_section = false;
    let mut fence = None;
    let mut visible = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let structural_line = markdown_structure(line);
        if let Some((marker, minimum)) = fence {
            if structural_line
                .and_then(fence_delimiter)
                .is_some_and(|(candidate, count, rest)| {
                    candidate == marker && count >= minimum && rest.trim().is_empty()
                })
            {
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some((marker, count, _)) = structural_line.and_then(fence_delimiter) {
            fence = Some((marker, count));
            index += 1;
            continue;
        }
        let setext_heading = lines
            .get(index + 1)
            .is_some_and(|next| is_setext_underline(next));
        let heading = structural_line.and_then(atx_heading).or_else(|| {
            structural_line
                .filter(|line| setext_heading && !line.is_empty())
                .map(str::trim)
        });
        if let Some(heading) = heading {
            historical_section = is_historical_heading(heading);
            visible.push(format!("<markdown-heading> {heading} </markdown-heading>"));
            index += usize::from(setext_heading) + 1;
            continue;
        }
        if !historical_section && structural_line.is_some() {
            visible.push(line.to_owned());
        }
        index += 1;
    }
    visible
        .join(" ")
        .to_ascii_lowercase()
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn fence_delimiter(line: &str) -> Option<(char, usize, &str)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let count = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (count >= 3).then(|| (marker, count, &line[count..]))
}

fn atx_heading(line: &str) -> Option<&str> {
    let count = line
        .chars()
        .take_while(|candidate| *candidate == '#')
        .count();
    if !(1..=6).contains(&count) {
        return None;
    }
    let rest = &line[count..];
    (rest.is_empty() || rest.starts_with(char::is_whitespace))
        .then(|| rest.trim().trim_end_matches('#').trim_end())
}

fn is_setext_underline(line: &str) -> bool {
    let Some(line) = markdown_structure(line) else {
        return false;
    };
    let line = line.trim();
    !line.is_empty()
        && (line.chars().all(|character| character == '=')
            || line.chars().all(|character| character == '-'))
}

fn markdown_structure(line: &str) -> Option<&str> {
    let mut columns = 0;
    for (index, character) in line.char_indices() {
        match character {
            ' ' => columns += 1,
            '\t' => columns += 4 - columns % 4,
            _ => return (columns <= 3).then(|| &line[index..]),
        }
        if columns > 3 {
            return None;
        }
    }
    Some("")
}

fn is_historical_heading(heading: &str) -> bool {
    let heading = heading.to_ascii_lowercase();
    let mut parts = heading.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or_default();
    let unnumbered = parts.next().filter(|_| {
        first.chars().any(|character| character.is_ascii_digit())
            && first.chars().all(|character| {
                character.is_ascii_digit() || matches!(character, '.' | '(' | ')' | ':' | '-')
            })
    });
    let title = unnumbered.unwrap_or(&heading);
    matches!(title, "history" | "historical")
        || title.starts_with("history ")
        || title.starts_with("historical ")
}
