use std::{fs, path::PathBuf};

use serde_yaml::{Mapping, Value};

use super::wiki_core_article::CanonicalDate;

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

fn mapping(source: &str) -> Option<Mapping> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, remainder) = source.split_once('\n')?;
    if opening.trim_end_matches('\r') != "---" {
        return None;
    }
    let mut end = 0;
    for line in remainder.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return serde_yaml::from_str::<Value>(&remainder[..end])
                .ok()
                .and_then(|value| match value {
                    Value::Mapping(mapping) => Some(mapping),
                    _ => None,
                });
        }
        end += line.len();
    }
    None
}

fn field<'a>(mapping: &'a Mapping, name: &str) -> Option<&'a Value> {
    mapping.get(Value::String(name.into()))
}
