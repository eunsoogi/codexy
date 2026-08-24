use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail, ensure};
use codexy_runtime::validation::{
    HandoffAuthority, IssuePrIdentity, StableHandoff, validate_handoff_batch,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Capsule {
    schema: String,
    consumer: Consumer,
    subject: String,
    source_task: String,
    target_task: String,
    replay_path: PathBuf,
    authority: Authority,
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
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let path = capsule_argument()?;
    let capsule: Capsule = serde_json::from_slice(
        &fs::read(&path).with_context(|| format!("reading capsule: {}", path.display()))?,
    )
    .context("invalid capsule JSON")?;
    ensure!(
        capsule.schema == "codexy.resumable-context-capsule.v1",
        "capsule schema"
    );
    let authority = HandoffAuthority::new(
        &capsule.authority.current_head,
        &capsule.authority.owner,
        &capsule.authority.worktree,
        IssuePrIdentity {
            issue: capsule.authority.issue,
            pr: capsule.authority.pr,
        },
        &capsule.authority.branch,
        &capsule.authority.base,
    )
    .with_stable(capsule.authority.stable.clone());
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

fn capsule_argument() -> Result<PathBuf> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 || arguments[0] != "--capsule" {
        bail!("usage: codexy-handoff-validate --capsule PATH");
    }
    Ok(PathBuf::from(&arguments[1]))
}

fn bind_direction(
    capsule: &Capsule,
    volatile: &codexy_runtime::validation::HandoffVolatile,
) -> Result<()> {
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

fn read_replay(path: &Path) -> Result<Vec<String>> {
    match fs::read(path) {
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes).context("invalid replay JSON")?;
            serde_json::from_value(value).context("replay state must be an envelope array")
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error).with_context(|| format!("reading replay: {}", path.display())),
    }
}

fn write_replay(path: &Path, replay: &[String]) -> Result<()> {
    let parent = path.parent().context("replay path has no parent")?;
    let temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(temporary.as_file(), replay)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| anyhow::anyhow!("persisting replay: {}", error.error))?;
    Ok(())
}
