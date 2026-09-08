use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::mcp::CancellationToken;

use super::validation::{authorize, authorize_parent, check_cancelled};
use super::{MAX_REPORTS, MAX_WAIT_MS, Store};

impl Store {
    fn consistent_snapshot_with_transition(
        &self,
        session_id: &str,
    ) -> Result<(
        super::model::Session,
        Vec<super::model::Event>,
        super::super::lock::LockGuard,
    )> {
        let (transition, state) = self.session_lock(session_id)?;
        let mut session = self.load_session(session_id)?;
        let events = self.reconcile_events(&mut session)?;
        drop(state);
        Ok((session, events, transition))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn wait_with_cancellation(
        &self,
        session_id: &str,
        token: &str,
        cursor: u64,
        max_reports: usize,
        timeout_ms: u64,
        cancellation: Option<&CancellationToken>,
    ) -> Result<Value> {
        if max_reports == 0 || max_reports > MAX_REPORTS {
            bail!("watcher maxReports must be between 1 and {MAX_REPORTS}");
        }
        if timeout_ms > MAX_WAIT_MS {
            bail!("watcher timeoutMs must be at most {MAX_WAIT_MS}");
        }
        let (session, _, transition) = self.consistent_snapshot_with_transition(session_id)?;
        authorize_parent(&session, token)?;
        check_cancelled(cancellation)?;
        if cursor > session.next_sequence {
            bail!("watcher cursor is ahead of the session");
        }
        let _wait_lock = super::super::lock::LockGuard::try_acquire(
            &self.session_dir(session_id)?.join("wait.lock"),
        )?
        .context("watcher already has an active waiter")?;
        drop(transition);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            check_cancelled(cancellation)?;
            let (session, events) = self.consistent_snapshot(session_id)?;
            if session.status == "cancelled" {
                return self.wait_result("cancelled", cursor, &session, Vec::new());
            }
            if crate::watcher::io::now_ms() >= session.expires_at_ms {
                return self.wait_result("expired", cursor, &session, Vec::new());
            }
            let pending = events
                .into_iter()
                .filter(|event| event.sequence > cursor)
                .take(max_reports)
                .collect::<Vec<_>>();
            if !pending.is_empty() {
                let next = pending.last().map_or(cursor, |event| event.sequence);
                return self.wait_result("event", next, &session, pending);
            }
            if timeout_ms == 0 || Instant::now() >= deadline {
                return self.wait_result("timeout", cursor, &session, Vec::new());
            }
            if let Some(cancellation) = cancellation {
                cancellation.wait(Duration::from_millis(25));
            } else {
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    pub(crate) fn health(&self, session_id: &str, token: &str) -> Result<Value> {
        let (session, _, _transition) = self.consistent_snapshot_with_transition(session_id)?;
        let actor = authorize(&session, token, false)?;
        let health = self.load_health(session_id)?;
        self.health_value(&session, &health, actor)
    }

    pub(crate) fn cancel(&self, session_id: &str, token: &str) -> Result<Value> {
        let _lock = self.session_lock(session_id)?;
        let mut session = self.load_session(session_id)?;
        authorize_parent(&session, token)?;
        if session.status == "cancelled" {
            return Ok(json!({ "status": "already_cancelled", "sessionId": session_id }));
        }
        "cancelled".clone_into(&mut session.status);
        self.write_session(&session)?;
        let mut health = self.load_health(session_id)?;
        "cancelled".clone_into(&mut health.watcher_state);
        self.write_health(session_id, &health)?;
        super::super::io::write_json(
            &self.session_dir(session_id)?.join("cancel.json"),
            &json!({ "cancelledAtMs": crate::watcher::io::now_ms() }),
        )?;
        Ok(json!({ "status": "cancelled", "sessionId": session_id }))
    }
}
