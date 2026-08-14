use std::process::Output;

use crate::support::{ReleaseFixtureCommand, ReleaseFixtureOutcome};

pub(super) fn assert_rejected(operation: &str, output: &Output) {
    ReleaseFixtureCommand::assert_outcome(operation, ReleaseFixtureOutcome::Failure, output);
}
