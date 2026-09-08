use std::collections::HashSet;

pub(super) struct Expected<'a> {
    pub(super) repository: &'a str,
    pub(super) owning_issue: u64,
    pub(super) pull_request: u64,
    pub(super) base: &'a str,
    pub(super) source_head: &'a str,
    pub(super) current_head: &'a str,
    pub(super) review_event_id: &'a str,
    pub(super) finding_ids: &'a HashSet<String>,
}

const TITLE: &str = "## Final parent disposition recorded by the release orchestrator";
const PREAMBLE: &str = "This records the parent-maintainer's bounded final disposition for the completed third review. The orchestrator records an existing decision; it does not create a reviewer event.";
const NON_WAIVER: &str = "This disposition does not waive CI, review-thread resolution, ownership, branch synchronization, connector review, or merge requirements; it does not authorize a fourth review, synthetic PASS, counter reset, or history rewrite.";
const DECISION: &str = "accept only the bounded final disposition after the authentic third BLOCK; preserve all reviewer history and independently verify ordinary gates.";
const PREFIXES: [&str; 10] = [
    "Repository: ",
    "Owning issue: #",
    "Pull request: #",
    "Base: ",
    "Source head: ",
    "Current head: ",
    "Review event: ",
    "Addressed findings: ",
    "Remaining findings: ",
    "Decision: ",
];

pub(super) fn check(body: &str, expected: &Expected<'_>) -> Result<(), String> {
    let lines = body.lines().collect::<Vec<_>>();
    if lines.iter().any(|line| line.contains("```")) {
        return Err("final disposition authority comment contains a fenced example".into());
    }
    let headings = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == "Scope of this disposition:").then_some(index))
        .collect::<Vec<_>>();
    let [heading] = headings.as_slice() else {
        return Err("final disposition authority comment must contain one bounded scope".into());
    };
    if lines.get(..*heading) != Some(&[TITLE, "", PREAMBLE, ""].as_slice()) {
        return Err("final disposition authority comment has an unsupported preamble".into());
    }
    let mut end = heading + 1;
    while end < lines.len() && !lines[end].is_empty() {
        end += 1;
    }
    let scoped = &lines[heading + 1..end];
    if scoped.len() != PREFIXES.len()
        || scoped
            .iter()
            .zip(PREFIXES)
            .any(|(line, prefix)| !line.starts_with(&format!("- {prefix}")))
    {
        return Err("final disposition authority comment has an invalid scope".into());
    }
    let tail = lines
        .get(end..)
        .unwrap_or_default()
        .iter()
        .filter(|line| !line.is_empty())
        .copied()
        .collect::<Vec<_>>();
    if tail.as_slice() != [NON_WAIVER] {
        return Err(
            "final disposition authority comment has an invalid non-waiver statement".into(),
        );
    }
    if field(scoped, "Repository: ")? != expected.repository
        || number(field(scoped, "Owning issue: #")?)? != expected.owning_issue
        || number(field(scoped, "Pull request: #")?)? != expected.pull_request
        || field(scoped, "Base: ")? != expected.base
        || field(scoped, "Source head: ")? != expected.source_head
        || field(scoped, "Current head: ")? != expected.current_head
        || field(scoped, "Review event: ")? != expected.review_event_id
        || field(scoped, "Remaining findings: ")? != "none"
        || field(scoped, "Decision: ")? != DECISION
    {
        return Err("final disposition authority comment does not bind exact final facts".into());
    }
    let addressed = parse_ids(field(scoped, "Addressed findings: ")?)?;
    if addressed != *expected.finding_ids {
        return Err("final disposition authority comment does not cover exact findings".into());
    }
    Ok(())
}

fn field<'a>(lines: &[&'a str], prefix: &str) -> Result<&'a str, String> {
    let matches = lines
        .iter()
        .filter_map(|line| line.strip_prefix("- ")?.strip_prefix(prefix))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [value] => Ok(*value),
        _ => Err(format!(
            "final disposition authority comment must contain one `{prefix}` field"
        )),
    }
}

fn number(value: &str) -> Result<u64, String> {
    value
        .parse()
        .ok()
        .filter(|number| *number > 0)
        .ok_or("final disposition authority comment contains an invalid number".into())
}

fn parse_ids(value: &str) -> Result<HashSet<String>, String> {
    let mut ids = HashSet::new();
    for id in value.split(',').map(str::trim) {
        if id.is_empty()
            || id.len() > 128
            || id
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte == b',')
            || !ids.insert(id.to_owned())
        {
            return Err("final disposition authority comment finding ids are invalid".into());
        }
    }
    Ok(ids)
}
