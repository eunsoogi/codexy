mod events;
mod model;
mod operations;
mod validation;
mod wait;

use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use self::model::{Event, Health, Session};
use super::io::{ensure_dir, now_ms, read_json, reject_link, safe_id, state_root, write_json};
use super::lock::LockGuard;

pub(crate) const MAX_TARGETS: usize = 8;
pub(crate) const MAX_EVENTS: usize = 64;
pub(crate) const MAX_SESSIONS: usize = 128;
pub(crate) const MAX_REPORTS: usize = 8;
pub(crate) const MAX_WAIT_MS: u64 = 30_000;
pub(crate) const MAX_TTL_SECONDS: u64 = 86_400;
const LOCK_WAIT_MS: u64 = 2_000;
pub(super) const EVENT_BYTES: usize = 16_384;

#[derive(Debug, Clone)]
pub(crate) struct Store {
    root: PathBuf,
}

impl Store {
    pub(crate) fn new() -> Result<Self> {
        let root = state_root()?;
        ensure_dir(&root)?;
        Ok(Self { root })
    }

    fn find_assignment(&self, assignment_id: &str) -> Result<Option<Session>> {
        for directory in self.session_dirs()? {
            let path = directory.join("session.json");
            if path.is_file() {
                let session: Session = read_json(&path, "session")?;
                if session.assignment_id == assignment_id {
                    return Ok(Some(session));
                }
            }
        }
        Ok(None)
    }

    fn session_dirs(&self) -> Result<Vec<PathBuf>> {
        let mut directories = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                reject_link(&path)?;
            }
            if file_type.is_dir() {
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow::anyhow!("watcher session id is not UTF-8"))?;
                safe_id(&name, "sessionId")?;
                directories.push(path);
            }
        }
        Ok(directories)
    }

    pub(super) fn reclaim_sessions(&self, now: u64) -> Result<()> {
        for directory in self.session_dirs()? {
            let session_path = directory.join("session.json");
            if !session_path.exists() {
                if fs::read_dir(&directory)?.next().is_none() {
                    fs::remove_dir(&directory)?;
                    continue;
                }
                bail!("watcher session directory is incomplete");
            }
            let Some(lock) = LockGuard::try_acquire(&directory.join("state.lock"))? else {
                continue;
            };
            let session: Session = read_json(&session_path, "session")?;
            if session.status != "active" || now >= session.expires_at_ms {
                remove_session_directory(&directory)?;
            }
            drop(lock);
        }
        Ok(())
    }

    pub(super) fn ensure_session_capacity(&self) -> Result<()> {
        let count = self
            .session_dirs()?
            .into_iter()
            .try_fold(0, |count, directory| {
                let session = directory.join("session.json");
                Ok::<_, anyhow::Error>(count + usize::from(session.is_file()))
            })?;
        if count >= MAX_SESSIONS {
            bail!("watcher session storage is full; wait for expiry before opening another");
        }
        Ok(())
    }

    fn session_dir(&self, session_id: &str) -> Result<PathBuf> {
        safe_id(session_id, "sessionId")?;
        Ok(self.root.join(session_id))
    }

    fn load_session(&self, session_id: &str) -> Result<Session> {
        let dir = self.session_dir(session_id)?;
        reject_link(&dir)?;
        read_json(&dir.join("session.json"), "session")
    }

    fn write_session(&self, session: &Session) -> Result<()> {
        write_json(
            &self.session_dir(&session.session_id)?.join("session.json"),
            &serde_json::to_value(session)?,
        )
    }

    fn session_lock(&self, session_id: &str) -> Result<LockGuard> {
        let dir = self.session_dir(session_id)?;
        ensure_dir(&dir)?;
        LockGuard::acquire(&dir.join("state.lock"), LOCK_WAIT_MS)
    }

    fn load_health(&self, session_id: &str) -> Result<Health> {
        let path = self.session_dir(session_id)?.join("health.json");
        if !path.exists() {
            return Ok(Health {
                watcher_state: "unknown".to_owned(),
                ..Health::default()
            });
        }
        read_json(&path, "health")
    }

    fn write_health(&self, session_id: &str, health: &Health) -> Result<()> {
        write_json(
            &self.session_dir(session_id)?.join("health.json"),
            &serde_json::to_value(health)?,
        )
    }

    pub(in crate::watcher::state) fn consistent_snapshot(
        &self,
        session_id: &str,
    ) -> Result<(Session, Vec<Event>)> {
        let _lock = self.session_lock(session_id)?;
        let mut session = self.load_session(session_id)?;
        let events = self.reconcile_events(&mut session)?;
        Ok((session, events))
    }

    pub(in crate::watcher::state) fn reconcile_events(
        &self,
        session: &mut Session,
    ) -> Result<Vec<Event>> {
        let events = events::read(&self.root, &session.session_id)?;
        let last_sequence = events.last().map_or(0, |event| event.sequence);
        if last_sequence > session.next_sequence {
            session.next_sequence = last_sequence;
            self.write_session(session)?;
        }
        if last_sequence != session.next_sequence {
            bail!("watcher session and event log are inconsistent");
        }
        Ok(events)
    }

    fn wait_result(
        &self,
        status: &str,
        cursor: u64,
        session: &Session,
        events: Vec<Event>,
    ) -> Result<Value> {
        let health = self.load_health(&session.session_id)?;
        Ok(json!({
            "status": status,
            "sessionId": session.session_id,
            "events": events,
            "nextCursor": cursor.to_string(),
            "health": self.health_value(session, &health, "parent")?,
        }))
    }

    fn health_value(&self, session: &Session, health: &Health, actor: &str) -> Result<Value> {
        let wait_path = self.session_dir(&session.session_id)?.join("wait.lock");
        if wait_path.exists() {
            reject_link(&wait_path)?;
        }
        let status = if session.status == "cancelled" {
            "cancelled"
        } else if now_ms() >= session.expires_at_ms {
            "expired"
        } else {
            "active"
        };
        Ok(json!({
            "status": status,
            "actor": actor,
            "sessionId": session.session_id,
            "assignmentId": session.assignment_id,
            "parent": session.parent,
            "watcher": session.watcher,
            "targets": session.targets,
            "generation": session.generation,
            "queueDepth": session.next_sequence,
            "lastObservationAtMs": health.last_observation_at_ms,
            "lastMaterialEventAtMs": health.last_material_event_at_ms,
            "watcherState": health.watcher_state,
            "lastError": health.last_error,
            "waiting": wait_path.exists(),
            "transport": "filesystem-queue",
            "transportConnected": true,
            "nativeStatus": "unverified",
            "expiresAtMs": session.expires_at_ms,
        }))
    }
}

fn remove_session_directory(path: &std::path::Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() {
            reject_link(&entry.path())?;
        }
    }
    fs::remove_dir_all(path).map_err(Into::into)
}
