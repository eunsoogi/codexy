use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodegraphErrorKind {
    RootMissing,
    RootInvalid,
    RootNotDirectory,
    RootUnreadable,
    FileDeletionRace,
    SourceMissing,
    PermissionDenied,
    EncodingFailure,
    ReadFailure,
    WalkFailure,
}

impl CodegraphErrorKind {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::RootMissing => "root_missing",
            Self::RootInvalid => "root_invalid",
            Self::RootNotDirectory => "root_not_directory",
            Self::RootUnreadable => "root_unreadable",
            Self::FileDeletionRace => "file_deletion_race",
            Self::SourceMissing => "source_missing",
            Self::PermissionDenied => "permission_denied",
            Self::EncodingFailure => "encoding_failure",
            Self::ReadFailure => "read_failure",
            Self::WalkFailure => "walk_failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CodegraphError {
    pub kind: CodegraphErrorKind,
    pub path: String,
    pub message: String,
}

impl CodegraphError {
    pub(super) fn new(
        kind: CodegraphErrorKind,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for CodegraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "codegraph {} at {}: {}",
            self.kind.as_str(),
            self.path,
            self.message
        )
    }
}

impl std::error::Error for CodegraphError {}

thread_local! {
    static ERRORS: RefCell<Vec<CodegraphError>> = const { RefCell::new(Vec::new()) };
    static DISCOVERED_FILES: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

pub(super) fn begin_operation() {
    ERRORS.with(|errors| errors.borrow_mut().clear());
    DISCOVERED_FILES.with(|files| files.borrow_mut().clear());
}

pub(super) fn take_errors() -> Vec<CodegraphError> {
    ERRORS.with(|errors| {
        let mut errors = errors.borrow_mut();
        errors.sort();
        errors.dedup();
        std::mem::take(&mut *errors)
    })
}

pub(super) fn record(error: CodegraphError) {
    ERRORS.with(|errors| errors.borrow_mut().push(error));
}

pub(super) fn remember_files(files: &[String]) {
    DISCOVERED_FILES.with(|known| known.borrow_mut().extend(files.iter().cloned()));
}

pub(super) fn was_discovered(file: &str) -> bool {
    DISCOVERED_FILES.with(|known| known.borrow().contains(file))
}
