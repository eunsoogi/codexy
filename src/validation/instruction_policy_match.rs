#[rustfmt::skip]
const PROHIBITION_MARKERS: &[&str] = &["do not", "don't", "avoid", "never", "shall not", "must not", "not allowed", "cannot"];
#[rustfmt::skip]
const MANDATORY_LINE_PREFIXES: &[&str] = &["act", "add", "append", "apply", "assign", "build", "capture", "check", "choose", "classify", "clone", "complete", "confirm", "continue", "create", "decide", "delete", "download", "drive", "establish", "extract", "fetch", "flag", "follow", "generate", "give", "identify", "include", "inspect", "keep", "list", "locate", "maintain", "mark", "move", "name", "open", "parse", "preflight", "preserve", "pull", "read", "re-read", "recalculate", "record", "regenerate", "remove", "report", "reproduce", "resolve", "route", "run", "re-run", "search", "separate", "separately", "skip", "stage", "start", "stop", "suggest", "test", "track", "treat", "update", "use", "verify", "walk", "write"];
#[rustfmt::skip]
const ROOT_AGENTS_PREFIXES: &[&str] = &["add", "capture", "keep", "mention", "preflight", "put", "treat", "wait"];
#[rustfmt::skip]
const PASSIVE_MANDATORY: &[&str] = &[" is required", " are required", " requires"];
pub(super) fn has_prohibition_without_must_not(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let inverted_list = has_forbidden_actions_without_must_not(line)
        || line.contains("MUST NOT")
            && [", MUST remove", ", MUST rewrite", ", MUST add"]
                .iter()
                .any(|needle| line.contains(needle));
    if inverted_list {
        return true;
    }
    if lower.match_indices("must not").any(|(index, _)| {
        !line[index..]
            .get(..8)
            .is_some_and(|candidate| candidate == "MUST NOT")
    }) {
        return true;
    }
    PROHIBITION_MARKERS.iter().any(|marker| {
        *marker != "must not"
            && crate::validation::instruction_policy_purpose::has_prohibition_marker(&lower, marker)
    })
}

pub(super) fn has_forbidden_actions_without_must_not(line: &str) -> bool {
    line.trim_start()
        .strip_prefix("Forbidden actions:")
        .is_some_and(|actions| !actions.trim_start().starts_with("MUST NOT"))
}

pub(super) fn starts_with_inverted_prohibition(line: &str) -> bool {
    let line = line.trim_start();
    ["MUST remove", "MUST rewrite", "MUST add"]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

pub(super) fn ends_with_dangling_modal(line: &str) -> bool {
    let line = line.trim_end();
    line.ends_with("MUST")
        || line.ends_with("MUST NOT")
        || has_modal_instruction(line) && (line.ends_with(" to") || line.ends_with(" from"))
}

#[rustfmt::skip]
pub(super) fn has_bare_mandatory_without_must(
    line: &str,
    strict_clauses: bool,
    root_agents: bool,
    custom_agent_toml: bool,
    passive_mandatory: bool,
) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.match_indices("must ").any(|(index, _)| {
        !line[index..]
            .get(..5)
            .is_some_and(|candidate| candidate == "MUST ")
    }) {
        return true;
    }
    if lower.starts_with("inspect first:") && !line.contains("MUST ") {
        return true;
    }
    if mandatory_segments(line, strict_clauses)
        .iter()
        .any(|segment| starts_with_bare_imperative(segment, strict_clauses))
        || passive_mandatory && !custom_agent_toml && line.split(", ").skip(1).chain(line.split(" — ").skip(1)).any(|segment| { let segment = segment.trim_start(); matches_prefix(&segment.to_ascii_lowercase(), &["add", "append", "update"]) && !segment.starts_with("MUST") })
    {
        return true;
    }
    if root_agents
        && std::iter::once(line)
            .chain(root_clause_segments(line))
            .any(|segment| {
                let segment = segment.trim_start();
                matches_prefix(&segment.to_ascii_lowercase(), ROOT_AGENTS_PREFIXES)
                    && !segment.starts_with("MUST")
            })
    {
        return true;
    }
    if custom_agent_toml
        && toml_clause_segments(line).iter().any(|segment| {
            let segment = segment.trim_start();
            matches_prefix(
                &segment.to_ascii_lowercase(),
                &[
                    "check", "compare", "handoff", "keep", "look", "provide", "replay", "return",
                    "stop",
                ],
            ) && !segment.starts_with("MUST")
        })
    {
        return true;
    }
    mandatory_segments(line, strict_clauses)
        .iter()
        .any(|segment| {
            starts_with_bare_require(segment)
                || passive_mandatory && has_bare_passive_mandatory(segment)
        })
}

fn root_clause_segments(line: &str) -> Vec<&str> {
    line.split(". ")
        .skip(1)
        .chain(line.split(", ").skip(1))
        .chain(line.split(" and ").skip(1))
        .map(str::trim)
        .collect()
}

fn toml_clause_segments(line: &str) -> Vec<&str> {
    let mut segments = vec![line];
    if let Some((label, rest)) = line.split_once(": ") {
        let label = label.to_ascii_lowercase();
        if label != "allowed actions" {
            segments.push(rest);
        }
    }
    segments.extend(line.split(". ").skip(1).map(str::trim));
    segments
}

fn mandatory_segments(line: &str, strict_clauses: bool) -> Vec<&str> {
    let mut segments = vec![line];
    if let Some((label, rest)) = line.split_once(": ") {
        let lower_label = label.trim_matches('*').to_ascii_lowercase();
        if !matches!(
            lower_label.as_str(),
            "allowed actions" | "description" | "output" | "role scope" | "short_description"
        ) {
            segments.push(rest);
        }
    }
    for segment in line
        .split(';')
        .skip(1)
        .chain(line.split(". ").skip(1))
        .chain(line.split(" then ").skip(1))
    {
        segments.push(segment.trim());
    }
    if strict_clauses {
        for segment in line.split(", ").skip(1).chain(line.split(" and ").skip(1)) {
            segments.push(segment.trim());
        }
    }
    segments
}

fn starts_with_bare_imperative(segment: &str, strict_clauses: bool) -> bool {
    let segment = segment
        .trim_start()
        .trim_start_matches('*')
        .trim_start_matches(['"', '\'']);
    if has_modal_label_prefix(segment) {
        return false;
    }
    let lower_owned = segment.to_ascii_lowercase();
    let lower = lower_owned.trim_start();
    let lower = lower.strip_prefix("and ").unwrap_or(lower);
    let lower = lower.strip_prefix("you ").unwrap_or(lower);
    let lower = lower.strip_prefix("the orchestrator ").unwrap_or(lower);
    let lower = lower.strip_prefix("orchestrator ").unwrap_or(lower);
    let lower = lower.split_once("?** ").map_or(lower, |(_, rest)| rest);
    if lower.starts_with("name:") || lower.starts_with("name =") {
        return false;
    }
    if lower.starts_with("stop condition") || lower.starts_with("stop/blocker") {
        return false;
    }
    if lower.contains(" should ") || lower.starts_with("should ") {
        return false;
    }
    matches_prefix(&lower, MANDATORY_LINE_PREFIXES) && !segment.starts_with("MUST")
        || strict_clauses && matches_prefix(&lower, &["split"])
}
fn starts_with_bare_require(segment: &str) -> bool {
    let segment = segment.trim_start().trim_start_matches(['"', '\'']);
    let lower_owned = segment.to_ascii_lowercase();
    let lower = lower_owned.trim_start();
    let lower = lower.strip_prefix("and ").unwrap_or(lower);
    let lower = lower.strip_prefix("you ").unwrap_or(lower);
    matches_prefix(lower, &["require"]) && !segment.starts_with("MUST")
}

fn has_modal_instruction(line: &str) -> bool {
    for (index, _) in line.match_indices("MUST") {
        if line[..index].matches('`').count() % 2 == 1 {
            continue;
        }
        if line[..index]
            .chars()
            .next_back()
            .is_some_and(|ch| !ch.is_whitespace() && !matches!(ch, ':' | ';' | '(' | '['))
        {
            continue;
        }
        let after = line[index + "MUST".len()..].trim_start();
        let after = after.strip_prefix("NOT ").unwrap_or(after);
        if matches_prefix(&after.to_ascii_lowercase(), MANDATORY_LINE_PREFIXES) {
            return true;
        }
    }
    false
}

fn has_modal_label_prefix(segment: &str) -> bool {
    segment.split_once(": ").is_some_and(|(label, rest)| {
        let label = label.trim_matches('*');
        let rest = rest.trim_start_matches(['"', '\'']);
        rest.starts_with("MUST")
            && label.split_whitespace().count() <= 4
            && !label.contains(['.', ',', ';'])
    })
}

fn has_bare_passive_mandatory(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    !segment.contains("MUST")
        && PASSIVE_MANDATORY
            .iter()
            .any(|marker| lower.contains(marker))
}

fn matches_prefix(lower: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        lower.strip_prefix(prefix).is_some_and(|rest| {
            rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with([':', '/'])
        })
    })
}
