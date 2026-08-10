use std::{fs, path::PathBuf};

use serde_yaml::{Mapping, Value};

use super::wiki_core_article::CanonicalDate;
use super::wiki_frontmatter::mapping;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum RawIngestionState {
    Missing,
    Malformed,
    Complete,
}

pub(crate) fn raw_ingestion(paths: &[PathBuf]) -> RawIngestionState {
    if paths.is_empty() {
        return RawIngestionState::Missing;
    }
    for path in paths {
        let Some(mapping) = fs::read_to_string(path)
            .ok()
            .and_then(|source| mapping(&source))
        else {
            return RawIngestionState::Malformed;
        };
        match field(&mapping, "ingested") {
            None => return RawIngestionState::Missing,
            Some(Value::String(value)) if CanonicalDate::parse(value).is_some() => {}
            Some(_) => return RawIngestionState::Malformed,
        }
    }
    RawIngestionState::Complete
}

fn field<'a>(mapping: &'a Mapping, name: &str) -> Option<&'a Value> {
    mapping.get(Value::String(name.into()))
}
