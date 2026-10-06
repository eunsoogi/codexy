#[cfg(unix)]
use serde_json::{Value, json};

// Typed variants constrain wait traces to approved events and value-free fields.
#[derive(Clone, Copy)]
pub(super) enum WaitTraceRecord {
    Received {
        binding_present: bool,
    },
    BindingClaim {
        binding_present: bool,
        attempted: bool,
        succeeded: bool,
    },
    Ended {
        cause: &'static str,
    },
    Failed {
        failure_class: &'static str,
    },
}

impl WaitTraceRecord {
    #[cfg(unix)]
    pub(super) fn to_value(self, timestamp_ms: u64) -> Option<Value> {
        match self {
            Self::Ended { cause }
                if !matches!(
                    cause,
                    "transport_cancelled"
                        | "session_cancelled"
                        | "request_binding_cancelled"
                        | "expired"
                        | "event"
                        | "timeout"
                ) =>
            {
                return None;
            }
            Self::Failed { failure_class }
                if !matches!(
                    failure_class,
                    "binding_claim" | "binding_marker" | "wait_error"
                ) =>
            {
                return None;
            }
            _ => {}
        }
        let event = match self {
            Self::Received { .. } => "wait_received",
            Self::BindingClaim { .. } => "binding_claim",
            Self::Ended { .. } => "wait_ended",
            Self::Failed { .. } => "wait_failed",
        };
        let mut value = json!({
            "timestampMs": timestamp_ms,
            "component": "wait",
            "event": event,
        });
        match self {
            Self::Received { binding_present } => {
                value["bindingPresent"] = json!(binding_present);
            }
            Self::BindingClaim {
                binding_present,
                attempted,
                succeeded,
            } => {
                value["bindingPresent"] = json!(binding_present);
                value["claimAttempted"] = json!(attempted);
                value["claimSucceeded"] = json!(succeeded);
            }
            Self::Ended { cause } => {
                value["endCause"] = json!(cause);
            }
            Self::Failed { failure_class } => {
                value["failureClass"] = json!(failure_class);
            }
        }
        Some(value)
    }
}
