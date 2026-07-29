use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use regex::Regex;
use serde::Serialize;

use super::files::{result_limit, walk_code_files};

pub(super) const SEARCH_MATCH_LIMIT_BYTES: usize = 2_048;
pub(super) const SEARCH_CONTENT_LIMIT_BYTES: usize = 8_192;

#[derive(Debug, Clone, Serialize)]
pub(super) struct SearchOutput {
    pub matches: Vec<String>,
    pub limits: SearchLimits,
    #[serde(rename = "returnedMatchCount")]
    pub returned_match_count: usize,
    pub truncation: SearchTruncation,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SearchLimits {
    #[serde(rename = "lineBytes")]
    line_bytes: usize,
    #[serde(rename = "contentBytes")]
    content_bytes: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct SearchTruncation {
    #[serde(rename = "resultCount")]
    result_count: bool,
    #[serde(rename = "lineBytes")]
    line_bytes: bool,
    #[serde(rename = "contentBytes")]
    content_bytes: bool,
}

pub(super) fn search(root: &Path, query: &str, limit: Option<usize>) -> Result<SearchOutput> {
    let pattern = Regex::new(query).with_context(|| format!("invalid search regex: {query}"))?;
    let mut output = SearchOutput {
        matches: Vec::new(),
        limits: SearchLimits {
            line_bytes: SEARCH_MATCH_LIMIT_BYTES,
            content_bytes: SEARCH_CONTENT_LIMIT_BYTES,
        },
        returned_match_count: 0,
        truncation: SearchTruncation::default(),
    };
    let result_limit = result_limit(limit);
    for file in walk_code_files(root) {
        let Ok(source) = fs::read_to_string(root.join(&file)) else {
            continue;
        };
        for (index, line) in source.lines().enumerate() {
            if !pattern.is_match(line) {
                continue;
            }
            if output.matches.len() >= result_limit {
                output.truncation.result_count = true;
                return Ok(output);
            }
            let (line, line_truncated) = truncate_utf8(
                &format!("./{file}:{}:{line}", index + 1),
                SEARCH_MATCH_LIMIT_BYTES,
            );
            if !push_if_content_fits(&mut output, line, line_truncated)? {
                return Ok(output);
            }
        }
    }
    Ok(output)
}

fn push_if_content_fits(
    output: &mut SearchOutput,
    line: String,
    line_truncated: bool,
) -> Result<bool> {
    let mut candidate = output.clone();
    candidate.matches.push(line);
    candidate.returned_match_count = candidate.matches.len();
    candidate.truncation.line_bytes |= line_truncated;
    if serde_json::to_vec(&candidate)?.len() > SEARCH_CONTENT_LIMIT_BYTES {
        output.truncation.line_bytes |= line_truncated;
        output.truncation.content_bytes = true;
        return Ok(false);
    }
    candidate.truncation.content_bytes = false;
    *output = candidate;
    Ok(true)
}

fn truncate_utf8(value: &str, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value.to_owned(), false);
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_owned(), true)
}
