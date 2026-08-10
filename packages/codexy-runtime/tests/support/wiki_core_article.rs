use std::path::{Component, Path, PathBuf};

use serde_yaml::{Mapping, Value};

use super::wiki_core_raw_ingestion::{RawIngestionState, raw_ingestion};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CanonicalDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CanonicalDate {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes
                .iter()
                .enumerate()
                .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        {
            return None;
        }
        let year = value[..4].parse().ok()?;
        let month = value[5..7].parse().ok()?;
        let day = value[8..].parse().ok()?;
        (year > 0 && month > 0 && month <= 12 && day > 0 && day <= days_in_month(year, month))
            .then_some(Self { year, month, day })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FreshnessState {
    Missing,
    Malformed,
    Future,
    Valid(CanonicalDate),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ProvenanceState {
    Missing,
    Malformed,
    Broken,
    Complete,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ArticleFinding {
    FutureFreshness,
}

#[derive(Debug)]
pub(crate) struct ArticleAssessment {
    pub(crate) freshness: FreshnessState,
    pub(crate) provenance: ProvenanceState,
    pub(crate) freshness_credit: u8,
    pub(crate) findings: Vec<ArticleFinding>,
    pub(crate) source_scalars: Vec<String>,
    pub(crate) resolved_sources: Vec<PathBuf>,
    pub(crate) raw_ingestion: RawIngestionState,
}

impl ArticleAssessment {
    pub(crate) fn blocks_migration(&self) -> bool {
        matches!(
            self.freshness,
            FreshnessState::Missing | FreshnessState::Malformed
        ) || !matches!(self.provenance, ProvenanceState::Complete)
            || !matches!(self.raw_ingestion, RawIngestionState::Complete)
    }
}

pub(crate) fn assess_article(
    article: &str,
    topic_root: &Path,
    evaluation_day: CanonicalDate,
) -> ArticleAssessment {
    let mapping = frontmatter(article)
        .and_then(|frontmatter| serde_yaml::from_str::<Value>(frontmatter).ok())
        .and_then(|value| match value {
            Value::Mapping(mapping) => Some(mapping),
            _ => None,
        });
    let Some(mapping) = mapping else {
        return ArticleAssessment {
            freshness: if has_frontmatter(article) {
                FreshnessState::Malformed
            } else {
                FreshnessState::Missing
            },
            provenance: if has_frontmatter(article) {
                ProvenanceState::Malformed
            } else {
                ProvenanceState::Missing
            },
            freshness_credit: 0,
            findings: Vec::new(),
            source_scalars: Vec::new(),
            resolved_sources: Vec::new(),
            raw_ingestion: RawIngestionState::Missing,
        };
    };
    let freshness = freshness(&mapping, evaluation_day);
    let (provenance, source_scalars, resolved_sources) = provenance(&mapping, topic_root);
    let raw_ingestion = matches!(provenance, ProvenanceState::Complete)
        .then(|| raw_ingestion(&resolved_sources))
        .unwrap_or(RawIngestionState::Missing);
    let future = matches!(freshness, FreshnessState::Future);
    ArticleAssessment {
        freshness_credit: u8::from(matches!(freshness, FreshnessState::Valid(_))) * 25,
        freshness,
        provenance,
        findings: future
            .then_some(ArticleFinding::FutureFreshness)
            .into_iter()
            .collect(),
        source_scalars,
        resolved_sources,
        raw_ingestion,
    }
}

fn has_frontmatter(article: &str) -> bool {
    article
        .strip_prefix('\u{feff}')
        .unwrap_or(article)
        .split_once('\n')
        .is_some_and(|(opening, _)| opening.trim_end_matches('\r') == "---")
}

fn frontmatter(article: &str) -> Option<&str> {
    let article = article.strip_prefix('\u{feff}').unwrap_or(article);
    let (opening, remainder) = article.split_once('\n')?;
    if opening.trim_end_matches('\r') != "---" {
        return None;
    }
    let mut end = 0;
    for line in remainder.split_inclusive('\n') {
        let marker = line.trim_end_matches(['\r', '\n']);
        if matches!(marker, "---" | "...") {
            return Some(&remainder[..end]);
        }
        end += line.len();
    }
    None
}

fn freshness(mapping: &Mapping, evaluation_day: CanonicalDate) -> FreshnessState {
    match field(mapping, "verified") {
        None => FreshnessState::Missing,
        Some(Value::String(value)) => match CanonicalDate::parse(value) {
            Some(date) if date > evaluation_day => FreshnessState::Future,
            Some(date) => FreshnessState::Valid(date),
            None => FreshnessState::Malformed,
        },
        Some(_) => FreshnessState::Malformed,
    }
}

fn provenance(
    mapping: &Mapping,
    topic_root: &Path,
) -> (ProvenanceState, Vec<String>, Vec<PathBuf>) {
    let Some(Value::Sequence(values)) = field(mapping, "sources") else {
        return (
            if field(mapping, "sources").is_none() {
                ProvenanceState::Missing
            } else {
                ProvenanceState::Malformed
            },
            Vec::new(),
            Vec::new(),
        );
    };
    if values.is_empty() {
        return (ProvenanceState::Malformed, Vec::new(), Vec::new());
    }
    let mut source_scalars = Vec::new();
    let mut resolved_sources = Vec::new();
    for value in values {
        let Some(source) = value.as_str() else {
            return (ProvenanceState::Malformed, Vec::new(), Vec::new());
        };
        let path = Path::new(source);
        if source.is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return (ProvenanceState::Malformed, Vec::new(), Vec::new());
        }
        source_scalars.push(source.into());
        resolved_sources.push(topic_root.join(path));
    }
    let state = if resolved_sources.iter().all(|path| path.is_file()) {
        ProvenanceState::Complete
    } else {
        ProvenanceState::Broken
    };
    (state, source_scalars, resolved_sources)
}

fn field<'a>(mapping: &'a Mapping, name: &str) -> Option<&'a Value> {
    mapping.get(Value::String(name.into()))
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
