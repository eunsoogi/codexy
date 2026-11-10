//! Export a validated, separate replay projection for an explicit rollback.

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail, ensure};
use codexy_runtime::validation::{HandoffAuthority, IssuePrIdentity, project_previous_replay};
use serde_json::json;
use sha2::{Digest as _, Sha256};

use super::{Authority, lock_replay};

pub(super) fn run() -> Result<()> {
    let args = std::env::args_os().skip(2).collect::<Vec<_>>();
    if args.len() != 6 {
        bail!("usage: --export-previous-replay --replay PATH --authority PATH --output PATH");
    }
    let mut replay = None;
    let mut authority = None;
    let mut output = None;
    for pair in args.chunks_exact(2) {
        match pair[0].to_str() {
            Some("--replay") if replay.is_none() => replay = Some(PathBuf::from(&pair[1])),
            Some("--authority") if authority.is_none() => authority = Some(PathBuf::from(&pair[1])),
            Some("--output") if output.is_none() => output = Some(PathBuf::from(&pair[1])),
            _ => bail!("invalid replay export arguments"),
        }
    }
    let replay = replay.context("missing replay")?;
    let authority = authority.context("missing authority")?;
    let output = output.context("missing output")?;
    ensure!(
        replay.is_absolute() && replay.canonicalize()? == replay,
        "replay must be an absolute canonical file"
    );
    ensure!(
        authority.is_absolute() && authority.canonicalize()? == authority,
        "authority must be an absolute canonical file"
    );
    let parent = output.parent().context("output has no parent")?;
    ensure!(
        output.is_absolute() && parent.canonicalize()? == parent && output != replay,
        "output must be a separate absolute canonical destination"
    );
    let trusted: Authority = serde_json::from_slice(&fs::read(&authority)?)?;
    ensure!(
        trusted.schema == "codexy.handoff-authority.v1",
        "authority schema"
    );
    let live = HandoffAuthority::new(
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
    let _lock = lock_replay(&replay)?;
    let source = fs::read(&replay)?;
    let entries: Vec<String> = serde_json::from_slice(&source).context("invalid replay JSON")?;
    ensure!(!entries.is_empty(), "replay is empty");
    let texts = entries.iter().map(String::as_str).collect::<Vec<_>>();
    let projected = project_previous_replay(&texts, &live)?;
    let bytes = serde_json::to_vec(&projected)?;
    ensure!(
        fs::read(&replay)? == source,
        "source replay changed during export"
    );
    write_new(parent, &output, &bytes)?;
    ensure!(fs::read(&output)? == bytes, "export readback failed");
    println!(
        "{}",
        json!({
            "schema": "codexy.previous-replay-export.v1", "count": projected.len(),
            "sourceSha256": format!("{:x}", Sha256::digest(&source)),
            "outputSha256": format!("{:x}", Sha256::digest(&bytes)),
        })
    );
    Ok(())
}

fn write_new(parent: &Path, output: &Path, bytes: &[u8]) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(output).map_err(|error| {
        anyhow::anyhow!("preserving existing export destination: {}", error.error)
    })?;
    Ok(())
}
