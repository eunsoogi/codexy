//! Implements session creation and report persistence. Session capabilities
//! are stored as hashes, and event fingerprints make client retries detectable.

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use super::model::{Event, Health, Session};
use super::validation::{
    authorize, ensure_live, same_open, validate_event_kind, validate_evidence, validate_identity,
    validate_target, validate_targets, validate_text, validate_watcher_state,
};
use super::{MAX_EVENTS, MAX_TTL_SECONDS, Store};
use crate::watcher::io::{canonical_text, hash_text, now_ms, random_hex, safe_id, token_hash};
use crate::watcher::state::events;

impl Store {
    pub(crate) fn open(
        &self,
        assignment_id: String,
        parent: Value,
        watcher: Value,
        targets: Vec<Value>,
        ttl_seconds: u64,
    ) -> Result<Value> {
        safe_id(&assignment_id, "assignmentId")?;
        validate_identity(&parent, "parent")?;
        validate_identity(&watcher, "watcher")?;
        validate_targets(&targets)?;
        if ttl_seconds == 0 || ttl_seconds > MAX_TTL_SECONDS {
            bail!("watcher ttlSeconds must be between 1 and {MAX_TTL_SECONDS}");
        }
        let _lock = super::super::lock::LockGuard::acquire(&self.root.join(".open.lock"), 2_000)?;
        self.reclaim_sessions(now_ms())?;
        if let Some(existing) = self.find_assignment(&assignment_id)? {
            if same_open(&existing, &parent, &watcher, &targets) {
                return Ok(json!({
                    "status": "existing",
                    "sessionId": existing.session_id,
                    "cursor": existing.next_sequence.to_string(),
                    "expiresAtMs": existing.expires_at_ms,
                }));
            }
            bail!("watcher assignmentId is already bound to another identity");
        }
        self.ensure_session_capacity()?;
        let created_at = now_ms();
        let expires = created_at.saturating_add(ttl_seconds.saturating_mul(1_000));
        let (session, parent_token, watcher_token) = loop {
            let session_id = random_hex(16)?;
            let dir = self.session_dir(&session_id)?;
            match std::fs::create_dir(&dir) {
                Ok(()) => {
                    super::super::io::ensure_dir(&dir)?;
                    let parent_token = random_hex(32)?;
                    let watcher_token = random_hex(32)?;
                    let session = Session {
                        schema: 1,
                        session_id,
                        assignment_id,
                        parent,
                        watcher,
                        targets,
                        generation: 1,
                        created_at_ms: created_at,
                        expires_at_ms: expires,
                        parent_token_hash: token_hash(&parent_token),
                        watcher_token_hash: token_hash(&watcher_token),
                        next_sequence: 0,
                        status: "active".to_owned(),
                    };
                    break (session, parent_token, watcher_token);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error).context("creating watcher session"),
            }
        };
        self.write_session(&session)?;
        self.write_health(
            &session.session_id,
            &Health {
                watcher_state: "starting".to_owned(),
                ..Health::default()
            },
        )?;
        Ok(json!({
            "status": "opened",
            "sessionId": session.session_id,
            "parentToken": parent_token,
            "watcherToken": watcher_token,
            "cursor": "0",
            "expiresAtMs": session.expires_at_ms,
            "generation": session.generation,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn report(
        &self,
        session_id: &str,
        token: &str,
        target: Value,
        kind: Option<String>,
        summary: Option<String>,
        evidence: Vec<String>,
        requested_event_id: Option<&str>,
        observed_at_ms: Option<u64>,
        watcher_state: Option<String>,
        last_error: Option<String>,
    ) -> Result<Value> {
        let _lock = self.session_lock(session_id)?;
        let mut session = self.load_session(session_id)?;
        authorize(&session, token, true)?;
        ensure_live(&session, now_ms())?;
        validate_target(&session, &target)?;
        let timestamp = observed_at_ms.unwrap_or_else(now_ms);
        let mut health = self.load_health(session_id)?;
        health.last_observation_at_ms = Some(timestamp);
        if let Some(state) = watcher_state {
            validate_watcher_state(&state)?;
            health.watcher_state = state;
        }
        if let Some(error) = &last_error {
            validate_text(error, "lastError", 1_024)?;
        }
        health.last_error = last_error;
        let Some(kind) = kind else {
            self.write_health(session_id, &health)?;
            return Ok(
                json!({ "status": "health_updated", "sessionId": session_id, "observedAtMs": timestamp }),
            );
        };
        validate_event_kind(&kind)?;
        let summary = summary.context("watcher material reports require summary")?;
        validate_text(&summary, "summary", 4_096)?;
        validate_evidence(&evidence)?;
        let target_key = canonical_text(&target)?;
        let existing_events = self.reconcile_events(&mut session)?;
        let last_target_event = existing_events.iter().rev().find(|event| {
            canonical_text(&event.target).is_ok_and(|candidate| candidate == target_key)
        });
        // Observation time is freshness metadata; only client ids make it part of retry identity.
        if requested_event_id.is_none() {
            if let Some(previous) = last_target_event.filter(|previous| {
                previous.kind.as_str() == kind.as_str()
                    && previous.summary.as_str() == summary.as_str()
                    && previous.evidence.as_slice() == evidence.as_slice()
            }) {
                self.write_health(session_id, &health)?;
                return Ok(json!({
                    "status": "duplicate",
                    "sessionId": session_id,
                    "eventId": previous.event_id,
                    "cursor": previous.sequence.to_string(),
                }));
            }
        }

        // The session prefix scopes the lane; the prior subject event versions later transitions.
        let seed = if let Some(id) = requested_event_id {
            safe_id(id, "eventId")?;
            format!("client|{id}")
        } else {
            let material_identity = json!({
                "subject": target_key,
                "version": last_target_event.map(|event| event.sequence),
                "kind": kind,
                "summary": summary,
                "evidence": evidence,
            });
            format!("material|{}", canonical_text(&material_identity)?)
        };
        let event_id = format!(
            "evt-{}",
            hash_text(&format!("{}|{seed}", session.session_id))
        );
        let mut fingerprint_value = json!({
            "eventId": event_id,
            "kind": kind,
            "target": target,
            "summary": summary,
            "evidence": evidence,
        });
        if requested_event_id.is_some() && observed_at_ms.is_some() {
            fingerprint_value["observedAtMs"] = json!(timestamp);
        }
        let fingerprint = hash_text(&canonical_text(&fingerprint_value)?);
        // Reusing an id is idempotent only when the full material report is unchanged.
        if let Some(previous) = existing_events
            .iter()
            .find(|event| event.event_id == event_id)
        {
            if previous.fingerprint == fingerprint {
                self.write_health(session_id, &health)?;
                return Ok(
                    json!({ "status": "duplicate", "sessionId": session_id, "eventId": event_id, "cursor": previous.sequence.to_string() }),
                );
            }
            bail!("watcher eventId conflicts with an existing report");
        }
        if existing_events.len() >= MAX_EVENTS {
            bail!("watcher event queue is full; report was not dropped");
        }
        let event = Event {
            event_id: event_id.clone(),
            sequence: session.next_sequence.saturating_add(1),
            kind,
            target,
            summary,
            observed_at_ms: timestamp,
            evidence,
            fingerprint,
        };
        events::append(&self.root, session_id, &event)?;
        session.next_sequence = event.sequence;
        self.write_session(&session)?;
        health.last_material_event_at_ms = Some(timestamp);
        self.write_health(session_id, &health)?;
        Ok(
            json!({ "status": "accepted", "sessionId": session_id, "eventId": event_id, "cursor": event.sequence.to_string() }),
        )
    }
}
