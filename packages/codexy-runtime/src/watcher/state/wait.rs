//! Reads a consistent event cursor and implements the single active long-poll
//! contract, including both MCP cancellation and cross-request bindings.

#[path = "wait_trace.rs"]
mod wait_trace;

use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::mcp::CancellationToken;

use super::validation::{authorize, authorize_parent, check_cancelled};
use super::{MAX_REPORTS, MAX_WAIT_MS, Store};
use wait_trace::{
    WaitTrace, finish_wait, record_binding_claim, record_wait_failure, record_wait_received,
};

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
        // Keep reclamation out until the caller has reserved the session's waiter slot.
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
        request_binding: Option<&str>,
    ) -> Result<Value> {
        let mut trace = WaitTrace::new();
        record_wait_received(&mut trace, request_binding.is_some());
        let result = (|| -> Result<Value> {
            if max_reports == 0 || max_reports > MAX_REPORTS {
                bail!("watcher maxReports must be between 1 and {MAX_REPORTS}");
            }
            if timeout_ms > MAX_WAIT_MS {
                bail!("watcher timeoutMs must be at most {MAX_WAIT_MS}");
            }
            let (session, _, transition) = self.consistent_snapshot_with_transition(session_id)?;
            authorize_parent(&session, token)?;
            if let Err(error) = check_cancelled(cancellation) {
                return finish_wait(&mut trace, Err(error), "transport_cancelled");
            }
            if cursor > session.next_sequence {
                bail!("watcher cursor is ahead of the session");
            }
            let _wait_lock = super::super::lock::LockGuard::try_acquire(
                &self.session_dir(session_id)?.join("wait.lock"),
            )?
            .context("watcher already has an active waiter")?;
            let _request_binding = match request_binding {
                Some(nonce) => {
                    match super::request_binding::claim(&self.root, nonce, session_id, token) {
                        Ok(guard) => {
                            record_binding_claim(&mut trace, true, true, true);
                            Some(guard)
                        }
                        Err(error) => {
                            record_wait_failure(&mut trace, "binding_claim");
                            return Err(error);
                        }
                    }
                }
                None => {
                    record_binding_claim(&mut trace, false, false, false);
                    None
                }
            };
            drop(transition);
            let deadline = Instant::now() + Duration::from_millis(timeout_ms);
            loop {
                if let Err(error) = check_cancelled(cancellation) {
                    return finish_wait(&mut trace, Err(error), "transport_cancelled");
                }
                let (session, events) = self.consistent_snapshot(session_id)?;
                if session.status == "cancelled" {
                    return finish_wait(
                        &mut trace,
                        self.wait_result("cancelled", cursor, &session, Vec::new()),
                        "session_cancelled",
                    );
                }
                if crate::watcher::io::now_ms() >= session.expires_at_ms {
                    return finish_wait(
                        &mut trace,
                        self.wait_result("expired", cursor, &session, Vec::new()),
                        "expired",
                    );
                }
                let pending = events
                    .into_iter()
                    .filter(|event| event.sequence > cursor)
                    .take(max_reports)
                    .collect::<Vec<_>>();
                if !pending.is_empty() {
                    let next = pending.last().map_or(cursor, |event| event.sequence);
                    return finish_wait(
                        &mut trace,
                        self.wait_result("event", next, &session, pending),
                        "event",
                    );
                }
                if let Some(nonce) = request_binding {
                    match super::request_binding::cancelled(&self.root, nonce) {
                        Ok(true) => {
                            return finish_wait(
                                &mut trace,
                                self.wait_result("cancelled", cursor, &session, Vec::new()),
                                "request_binding_cancelled",
                            );
                        }
                        Ok(false) => {}
                        Err(error) => {
                            record_wait_failure(&mut trace, "binding_marker");
                            return Err(error);
                        }
                    }
                }
                if timeout_ms == 0 || Instant::now() >= deadline {
                    return finish_wait(
                        &mut trace,
                        self.wait_result("timeout", cursor, &session, Vec::new()),
                        "timeout",
                    );
                }
                if let Some(cancellation) = cancellation {
                    cancellation.wait(Duration::from_millis(25));
                } else {
                    thread::sleep(Duration::from_millis(25));
                }
            }
        })();
        if result.is_err() && trace.as_ref().is_some_and(|trace| !trace.is_finished()) {
            record_wait_failure(&mut trace, "wait_error");
        }
        result
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
