use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{Event, Session, Store};

// The response cause comes from the branch that ended the wait, never from health timestamps.
#[derive(Clone, Copy)]
pub(super) enum WaitCancellationReason {
    RequestBinding,
    Session,
}

impl WaitCancellationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::RequestBinding => "request_cancelled",
            Self::Session => "session_cancelled",
        }
    }
}

impl Store {
    pub(super) fn wait_result(
        &self,
        status: &str,
        cancellation_reason: Option<WaitCancellationReason>,
        cursor: u64,
        session: &Session,
        events: Vec<Event>,
    ) -> Result<Value> {
        if (status == "cancelled") != cancellation_reason.is_some() {
            bail!("watcher wait cancellation result has an invalid cause");
        }
        let health = self.load_health(&session.session_id)?;
        let mut result = json!({
            "status": status,
            "sessionId": session.session_id,
            "events": events,
            "nextCursor": cursor.to_string(),
            "health": {
                "status": Self::health_status(session),
                "watcherState": health.watcher_state,
                "lastObservationAtMs": health.last_observation_at_ms,
                "lastError": health.last_error,
            },
        });
        if let Some(reason) = cancellation_reason {
            result["cancellationReason"] = json!(reason.as_str());
        }
        Ok(result)
    }

    // Keep the compact wait summary and detailed diagnostic status in sync.
    pub(super) fn health_status(session: &Session) -> &'static str {
        if session.status == "cancelled" {
            "cancelled"
        } else if crate::watcher::io::now_ms() >= session.expires_at_ms {
            "expired"
        } else {
            "active"
        }
    }
}
