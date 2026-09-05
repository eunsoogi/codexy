use std::ops::Range;
use std::path::Path;

use anyhow::{Context as _, Result};
use regex::Regex;
use serde::Serialize;

use super::errors::{CodegraphError, begin_operation, take_errors};
use super::files::{read_source, result_limit, walk_code_files};

pub(super) const SEARCH_MATCH_LIMIT_BYTES: usize = 2_048;
pub(super) const SEARCH_CONTENT_LIMIT_BYTES: usize = 8_192;

#[derive(Debug, Clone, Serialize)]
pub(super) struct SearchOutput {
    pub matches: Vec<String>,
    pub limits: SearchLimits,
    #[serde(rename = "returnedMatchCount")]
    pub returned_match_count: usize,
    pub truncation: SearchTruncation,
    #[serde(skip_serializing_if = "is_false")]
    pub partial: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CodegraphError>,
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
    begin_operation();
    let pattern = Regex::new(query).with_context(|| format!("invalid search regex: {query}"))?;
    let mut output = SearchOutput {
        matches: Vec::new(),
        limits: SearchLimits {
            line_bytes: SEARCH_MATCH_LIMIT_BYTES,
            content_bytes: SEARCH_CONTENT_LIMIT_BYTES,
        },
        returned_match_count: 0,
        truncation: SearchTruncation::default(),
        partial: false,
        errors: Vec::new(),
    };
    let result_limit = result_limit(limit);
    for file in walk_code_files(root) {
        let source = read_source(root, &file);
        for (index, line) in source.lines().enumerate() {
            let Some(found) = pattern.find(line) else {
                continue;
            };
            if output.matches.len() >= result_limit {
                output.truncation.result_count = true;
                return Ok(finish(output));
            }
            let formatted = format!("./{file}:{}:{line}", index + 1);
            let line_start = formatted.len() - line.len();
            let (line, line_truncated) = truncate_utf8_around(
                &formatted,
                (line_start + found.start())..(line_start + found.end()),
                SEARCH_MATCH_LIMIT_BYTES,
            );
            if !push_if_content_fits(&mut output, line, line_truncated)? {
                return Ok(finish(output));
            }
        }
    }
    Ok(finish(output))
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn finish(mut output: SearchOutput) -> SearchOutput {
    output.errors = take_errors();
    output.partial = !output.errors.is_empty();
    bound_content(&mut output);
    output
}

fn bound_content(output: &mut SearchOutput) {
    if serialized_size(output) <= SEARCH_CONTENT_LIMIT_BYTES {
        return;
    }
    output.truncation.content_bytes = true;
    while output.errors.len() > 1 && serialized_size(output) > SEARCH_CONTENT_LIMIT_BYTES {
        output.errors.pop();
    }
    if serialized_size(output) > SEARCH_CONTENT_LIMIT_BYTES {
        if let Some(error) = output.errors.first_mut() {
            error.path = ".".to_owned();
            error.message = "error details truncated".to_owned();
        }
    }
    while serialized_size(output) > SEARCH_CONTENT_LIMIT_BYTES && !output.matches.is_empty() {
        output.matches.pop();
        output.returned_match_count = output.matches.len();
    }
    if serialized_size(output) > SEARCH_CONTENT_LIMIT_BYTES {
        output.errors.clear();
    }
}

fn serialized_size(output: &SearchOutput) -> usize {
    serde_json::to_vec(output).map_or(usize::MAX, |payload| payload.len())
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

fn truncate_utf8_around(value: &str, matched: Range<usize>, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value.to_owned(), false);
    }
    let context_before = limit.saturating_sub(matched.len()) / 2;
    let mut start = matched
        .start
        .saturating_sub(context_before)
        .min(value.len() - limit);
    while !value.is_char_boundary(start) {
        start += 1;
    }
    let mut end = start + limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[start..end].to_owned(), true)
}
