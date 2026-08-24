use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail, ensure};
use codexy_runtime::validation::{
    HandoffAuthority, HandoffVolatile, IssuePrIdentity, StableHandoff, validate_handoff_batch,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(unix)]
use std::os::fd::AsRawFd as _;
#[cfg(windows)]
use std::{os::windows::fs::OpenOptionsExt as _, thread, time::Duration};

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
    if let Err(error) = run() {
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

fn lock_replay(path: &Path) -> Result<File> {
    let name = path.file_name().context("replay path has no file name")?;
    let lock_path = path.with_file_name(format!("{}.lock", name.to_string_lossy()));
    let lock = open_lock(&lock_path)?;
    ensure!(
        fs::symlink_metadata(&lock_path)?.file_type().is_file(),
        "replay lock must be a regular file"
    );
    Ok(lock)
}

fn lock_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    options
}

#[cfg(unix)]
fn open_lock(path: &Path) -> Result<File> {
    let lock = lock_options()
        .open(path)
        .with_context(|| format!("opening replay lock: {}", path.display()))?;
    // SAFETY: flock borrows this live descriptor; the returned File holds the lock.
    let status = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) };
    if status == 0 {
        Ok(lock)
    } else {
        Err(std::io::Error::last_os_error())
            .with_context(|| format!("locking replay: {}", path.display()))
    }
}

#[cfg(windows)]
fn open_lock(path: &Path) -> Result<File> {
    loop {
        match lock_options().share_mode(0).open(path) {
            Ok(file) => return Ok(file),
            Err(error) if matches!(error.raw_os_error(), Some(32 | 33)) => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("opening replay lock: {}", path.display()));
            }
        }
    }
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
