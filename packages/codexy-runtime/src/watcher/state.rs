mod events;
mod model;
mod operations;
mod validation;
mod wait;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};

use self::model::{Event, Health, Session};
use super::io::{ensure_dir, now_ms, read_json, reject_link, safe_id, state_root, write_json};
use super::lock::LockGuard;

pub(crate) const MAX_TARGETS: usize = 8;
pub(crate) const MAX_EVENTS: usize = 64;
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
        for entry in fs::read_dir(&self.root)?.take(128) {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path().join("session.json");
            if path.is_file() {
                let session: Session = read_json(&path, "session")?;
                if session.assignment_id == assignment_id {
                    return Ok(Some(session));
                }
            }
        }
        Ok(None)
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
