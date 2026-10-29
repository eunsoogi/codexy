// Public examples remain repository test data. The harness checks structure,
// while focused assertions preserve each corpus's contract semantics.
#[path = "skill_public_corpora_checks.rs"]
mod checks;
#[path = "skill_public_corpora_prune_checks.rs"]
mod prune_checks;

use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result, anyhow, ensure};

type TestResult = Result<()>;

#[derive(Debug)]
struct Case {
    id: String,
    classification: String,
    body: String,
}

#[test]
fn public_skill_corpora_keep_coverage_and_structured_semantics() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let fixtures = [
        (
            "packages/codexy-runtime/tests/fixtures/blind_read_public_cases.md",
            "BR",
        ),
        (
            "packages/codexy-runtime/tests/fixtures/project_brief_public_cases.md",
            "PB",
        ),
        (
            "packages/codexy-runtime/tests/fixtures/prune_artifact_claims_public_cases.md",
            "AR",
        ),
    ];

    for (path, prefix) in fixtures {
        let relative = Path::new(path);
        ensure!(relative.starts_with("packages/codexy-runtime/tests/fixtures"));
        ensure!(!relative.starts_with("plugins/codexy"));
        ensure!(root.join(relative).is_file(), "missing {path}");
        let cases = read_cases(root, relative, prefix)?;
        match prefix {
            "BR" => checks::check_blind_read(&cases)?,
            "PB" => checks::check_project_brief(&cases)?,
            "AR" => prune_checks::check_prune_artifact_claims(&cases)?,
            _ => return Err(anyhow!("unknown fixture prefix: {prefix}")),
        }
    }

    Ok(())
}

#[test]
fn record_parser_splits_only_outside_inline_code() -> TestResult {
    // Exercise both separators together without adding another skill case.
    let case = Case {
        id: "record-parser".to_owned(),
        classification: "FORMAT".to_owned(),
        body: "- sample: phase=`A; B`; owner=`C`\n".to_owned(),
    };
    let (keys, fields) = record(&case, "sample")?;

    ensure!(
        keys.iter().map(String::as_str).eq(["phase", "owner"]),
        "record separators inside code spans changed field order"
    );
    ensure!(
        fields.get("phase").map(String::as_str) == Some("A; B"),
        "semicolon inside inline-code value was treated as a separator"
    );
    ensure!(
        fields.get("owner").map(String::as_str) == Some("C"),
        "field after inline-code value was not retained"
    );
    Ok(())
}

fn read_cases(root: &Path, path: &Path, prefix: &str) -> Result<Vec<Case>> {
    let source = fs::read_to_string(root.join(path))?;
    let mut cases = Vec::new();
    let mut current: Option<Case> = None;

    for line in source.lines() {
        if let Some(heading) = line.strip_prefix("### ") {
            if let Some(case) = current.take() {
                cases.push(case);
            }
            let (id, classification) = heading
                .split_once(" | ")
                .ok_or_else(|| anyhow!("invalid case heading: {heading}"))?;
            current = Some(Case {
                id: id.to_owned(),
                classification: classification.to_owned(),
                body: String::new(),
            });
        } else if let Some(case) = current.as_mut() {
            case.body.push_str(line);
            case.body.push('\n');
        }
    }
    if let Some(case) = current {
        cases.push(case);
    }

    let mut expected_ids = Vec::with_capacity(20);
    for (kind, classification) in [("P", "POSITIVE"), ("N", "NEGATIVE")] {
        for number in 1..=10 {
            expected_ids.push((format!("{prefix}-{kind}{number:02}"), classification));
        }
    }
    ensure!(
        cases.len() == expected_ids.len(),
        "unexpected case count in {path:?}"
    );
    for (case, (expected_id, expected_classification)) in cases.iter().zip(expected_ids) {
        ensure!(case.id == expected_id, "case order changed in {path:?}");
        ensure!(
            case.classification == expected_classification,
            "case classification changed for {}",
            case.id
        );
    }

    Ok(cases)
}

fn field_names(case: &Case) -> Vec<&str> {
    case.body
        .lines()
        .filter_map(|line| line.strip_prefix("- "))
        .filter_map(|line| line.split_once(": ").map(|(name, _)| name))
        .collect()
}

fn ensure_field_order(case: &Case, expected: &[&str]) -> Result<()> {
    ensure!(
        field_names(case).as_slice() == expected,
        "field order changed in {}",
        case.id
    );
    Ok(())
}

fn field(case: &Case, name: &str) -> Result<String> {
    let marker = format!("- {name}: ");
    let lines = case.body.lines().collect::<Vec<_>>();
    let index = lines
        .iter()
        .position(|line| line.starts_with(&marker))
        .ok_or_else(|| anyhow!("{} is missing {name}", case.id))?;
    let mut value = lines[index]
        .strip_prefix(&marker)
        .context("field marker disappeared")?
        .trim()
        .to_owned();

    for continuation in lines.iter().skip(index + 1) {
        if !continuation.starts_with("  ") {
            break;
        }
        value.push(' ');
        value.push_str(continuation.trim());
    }

    Ok(value)
}

fn scalar_field(case: &Case, name: &str) -> Result<String> {
    let value = field(case, name)?;
    // A scalar can be one inline-code span; composite records must wait until
    // after their semicolon-delimited values have been split.
    let value = value
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or(&value);
    Ok(value.to_owned())
}

// Corpus records use semicolons as field delimiters. Join Markdown soft wraps
// first, then remove inline-code delimiters from individual record values.
fn record(case: &Case, name: &str) -> Result<(Vec<String>, HashMap<String, String>)> {
    let mut keys = Vec::new();
    let mut fields = HashMap::new();
    let source = field(case, name)?;
    for part in record_parts(case, name, &source)? {
        let (key, value) = part
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid {name} field in {}", case.id))?;
        let value = value
            .strip_prefix('`')
            .and_then(|value| value.strip_suffix('`'))
            .unwrap_or(value);
        ensure!(
            fields.insert(key.to_owned(), value.to_owned()).is_none(),
            "duplicate {key} field in {}",
            case.id
        );
        keys.push(key.to_owned());
    }
    Ok((keys, fields))
}

// A semicolon separates fields only outside a code span, where it can be data.
fn record_parts<'a>(case: &Case, name: &str, source: &'a str) -> Result<Vec<&'a str>> {
    let mut parts = Vec::new();
    let mut in_code = false;
    let mut start = 0;

    for (index, character) in source.char_indices() {
        if character == '`' {
            in_code = !in_code;
        } else if character == ';'
            && !in_code
            && source
                .get(index + 1..)
                .is_some_and(|suffix| suffix.starts_with(' '))
        {
            parts.push(&source[start..index]);
            start = index + 2;
        }
    }

    ensure!(
        !in_code,
        "unbalanced inline-code delimiters in {name} field of {}",
        case.id
    );
    parts.push(&source[start..]);
    Ok(parts)
}
