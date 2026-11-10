//! Validates handoff capsules against trusted lane authority and replay history.

use std::{fs, path::PathBuf};

use anyhow::{Context as _, Result, bail, ensure};
use codexy_runtime::validation::{
    HandoffAuthority, HandoffVolatile, IssuePrIdentity, StableHandoff, validate_handoff_batch,
};
use serde::{Deserialize, Serialize};

#[path = "handoff_replay/export.rs"]
mod replay_export;
#[path = "handoff_replay/storage.rs"]
mod replay_storage;
use replay_storage::{lock_replay, read_replay, write_replay};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Capsule {
    schema: String,
    consumer: Consumer,
    subject: String,
    source_task: String,
    target_task: String,
    replay_path: PathBuf,
    envelope: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Consumer {
    Compaction,
    FreshChild,
    ParentHandoff,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Authority {
    schema: String,
    current_head: String,
    owner: String,
    worktree: String,
    issue: Option<u64>,
    pr: Option<u64>,
    branch: String,
    base: String,
    stable: StableHandoff,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Response<'a> {
    schema: &'static str,
    status: &'static str,
    consumer: Consumer,
    subject: &'a str,
    volatile_identity: &'a str,
}

fn main() {
    let result = if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--export-previous-replay"))
    {
        replay_export::run()
    } else {
        run()
    };
    if let Err(error) = result {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let (path, authority_path) = arguments()?;
    let capsule: Capsule = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("reading capsule: {}", path.display()))?,
    )
    .context("invalid capsule JSON")?;
    ensure!(
        capsule.schema == "codexy.resumable-context-capsule.v1",
        "capsule schema"
    );
    let trusted: Authority = serde_json::from_slice(
        &fs::read(&authority_path)
            .with_context(|| format!("reading authority: {}", authority_path.display()))?,
    )
    .context("invalid authority JSON")?;
    ensure!(
        trusted.schema == "codexy.handoff-authority.v1",
        "authority schema"
    );
    let authority = HandoffAuthority::new(
        &trusted.current_head,
        &trusted.owner,
        &trusted.worktree,
        IssuePrIdentity {
            issue: trusted.issue,
            pr: trusted.pr,
        },
        &trusted.branch,
        &trusted.base,
    )
    .with_stable(trusted.stable);
    // Keep the lock across read, validation, and replacement so concurrent runs
    // cannot both validate against the same replay history.
    let _lock = lock_replay(&capsule.replay_path)?;
    let mut replay = read_replay(&capsule.replay_path)?;
    replay.push(capsule.envelope.clone());
    let texts = replay.iter().map(String::as_str).collect::<Vec<_>>();
    let envelopes = validate_handoff_batch(&texts, &authority)?;
    let validated = envelopes
        .last()
        .context("validated capsule batch is empty")?;
    bind_direction(&capsule, &validated.volatile)?;
    write_replay(&capsule.replay_path, &replay)?;
    println!(
        "{}",
        serde_json::to_string(&Response {
            schema: "codexy.handoff-validation.v1",
            status: "validated",
            consumer: capsule.consumer,
            subject: &capsule.subject,
            volatile_identity: &validated.volatile_identity,
        })?
    );
    Ok(())
}

fn arguments() -> Result<(PathBuf, PathBuf)> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        bail!("usage: codexy-handoff-validate --capsule PATH --authority PATH");
    }
    let mut capsule = None;
    let mut authority = None;
    for pair in arguments.chunks_exact(2) {
        match pair[0].to_str() {
            Some("--capsule") if capsule.is_none() => capsule = Some(PathBuf::from(&pair[1])),
            Some("--authority") if authority.is_none() => {
                authority = Some(PathBuf::from(&pair[1]));
            }
            _ => bail!("usage: codexy-handoff-validate --capsule PATH --authority PATH"),
        }
    }
    Ok((
        capsule.context("missing --capsule")?,
        authority.context("missing --authority")?,
    ))
}

fn bind_direction(capsule: &Capsule, volatile: &HandoffVolatile) -> Result<()> {
    let event = &volatile.event;
    ensure!(event.subject == capsule.subject, "capsule subject binding");
    let child = volatile
        .child_task
        .as_deref()
        .context("child task binding")?;
    let parent = volatile
        .parent_task
        .as_deref()
        .context("parent task binding")?;
    let expected = match capsule.consumer {
        Consumer::Compaction => ("compaction-resume", "compaction", parent, child, child),
        Consumer::FreshChild => (
            "fresh-child-continuation",
            "fresh-child",
            parent,
            child,
            child,
        ),
        Consumer::ParentHandoff => ("parent-handoff", "parent-handoff", child, parent, parent),
    };
    ensure!(
        event.kind == expected.0
            && event.lane == expected.1
            && capsule.source_task == expected.2
            && capsule.target_task == expected.3
            && capsule.subject == expected.4,
        "directional consumer and parent/child role binding"
    );
    Ok(())
}
