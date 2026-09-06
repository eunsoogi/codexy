pub(super) struct Projection {
    pub(super) finding_id: String,
    pub(super) finding_path: String,
    pub(super) actual_model: String,
    pub(super) actual_reasoning_effort: String,
}

const OPERATIVE_PREFIXES: [&str; 8] = [
    "Repository: ",
    "Owning issue: #",
    "Pull request: #",
    "Base: ",
    "Head: ",
    "Finding: ",
    "Finding path: ",
    "Accepted difference: ",
];
const ACCEPTED_PREFIX: &str = "the retained Sentinel's actual ";
const ACCEPTED_SUFFIX: &str = " execution may stand despite the planned newer model routing. Preserve the actual native reviewer identity, runtime model, verdicts and review count; do not relabel execution or repeat review solely for the model difference.";
const NON_WAIVER: &str = "This disposition accepts only that model-policy difference for the bound review history. It does not accept code defects, waive CI or review findings, authorize merge, reset review counters, or authorize a fourth review. Exact-head CI and the remaining code/source-provenance repair must be independently established. Future source validation must reread this comment and verify its identity, repository authority and exact scope.";

pub(super) fn parse(
    body: &str,
    repository: &str,
    owning_issue: u64,
    pull_request: u64,
    live_base: &str,
    live_head: &str,
) -> Result<Projection, String> {
    let lines = body.lines().collect::<Vec<_>>();
    if lines.iter().any(|line| line.contains("```")) {
        return Err("maintainer decision body contains a fenced decision example".into());
    }
    let headings = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == "Scope of this disposition:").then_some(index))
        .collect::<Vec<_>>();
    let [heading] = headings.as_slice() else {
        return Err("maintainer decision body must contain one bounded disposition section".into());
    };
    let mut end = heading + 1;
    while end < lines.len() && !lines[end].is_empty() {
        if !lines[end].starts_with("- ")
            || lines[end].contains("```")
            || lines[end].starts_with('>')
        {
            return Err("maintainer decision disposition section is not an operative list".into());
        }
        end += 1;
    }
    if end == heading + 1 {
        return Err("maintainer decision disposition section is empty".into());
    }
    let scoped = &lines[heading + 1..end];
    if scoped.len() != OPERATIVE_PREFIXES.len()
        || scoped.iter().any(|line| {
            line.strip_prefix("- ").is_none_or(|line| {
                !OPERATIVE_PREFIXES
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
            })
        })
    {
        return Err(
            "maintainer decision disposition section contains an unsupported operative field"
                .into(),
        );
    }
    let non_waiver_lines = lines
        .get(end..)
        .map(|tail| {
            tail.iter()
                .filter(|line| !line.is_empty())
                .copied()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match non_waiver_lines.as_slice() {
        [line] if *line == NON_WAIVER => {}
        [] => return Err("maintainer decision body is missing its non-waiver statement".into()),
        _ => {
            return Err(
                "maintainer decision body must contain exactly one authoritative non-waiver statement"
                    .into(),
            );
        }
    }
    let repository_line = line(scoped, "Repository: ")?;
    let issue_line = line(scoped, "Owning issue: #")?;
    let pr_line = line(scoped, "Pull request: #")?;
    let base_line = line(scoped, "Base: ")?;
    let head_line = line(scoped, "Head: ")?;
    let finding_id = line(scoped, "Finding: ")?;
    let finding_path = line(scoped, "Finding path: ")?;
    if repository_line != repository
        || issue_line.parse::<u64>().ok() != Some(owning_issue)
        || pr_line.parse::<u64>().ok() != Some(pull_request)
        || !is_oid(base_line)
        || !is_oid(head_line)
        || base_line != live_base
        || head_line != live_head
        || finding_path.starts_with('/')
        || finding_path.contains('\\')
        || finding_path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(
            "maintainer decision body does not bind its exact repository, issue, PR, refs, or path"
                .into(),
        );
    }
    let accepted = line(scoped, "Accepted difference: ")?;
    let tuple = accepted
        .strip_prefix(ACCEPTED_PREFIX)
        .and_then(|value| value.strip_suffix(ACCEPTED_SUFFIX))
        .ok_or("maintainer decision body must use the operative accepted-difference sentence")?;
    let (actual_model, actual_reasoning_effort) = tuple
        .split_once('/')
        .filter(|(model, effort)| {
            model.starts_with("gpt-")
                && !model.is_empty()
                && !model.chars().any(char::is_whitespace)
                && !effort.is_empty()
                && !effort.contains('/')
                && !effort.chars().any(char::is_whitespace)
        })
        .ok_or("maintainer decision body must name the accepted model tuple")?;
    Ok(Projection {
        finding_id: finding_id.to_owned(),
        finding_path: finding_path.to_owned(),
        actual_model: actual_model.to_owned(),
        actual_reasoning_effort: actual_reasoning_effort.to_owned(),
    })
}

fn line<'a>(lines: &[&'a str], prefix: &str) -> Result<&'a str, String> {
    let matches = lines
        .iter()
        .filter_map(|line| line.strip_prefix("- ")?.strip_prefix(prefix))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [value] => Ok(*value),
        [] => Err(format!("maintainer decision body is missing `{prefix}`")),
        _ => Err(format!("maintainer decision body repeats `{prefix}`")),
    }
}

fn is_oid(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
