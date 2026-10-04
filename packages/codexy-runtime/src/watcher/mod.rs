mod io;
mod lock;
mod state;
pub mod tools;
mod tools_schema;

pub use tools::{call_tool, call_tool_with_cancellation};
pub use tools_schema::tools;

use anyhow::Result;
use serde_json::{Value, json};

#[must_use]
pub fn server_name() -> &'static str {
    "codexy-watcher"
}

// Arm one host invocation before watcher_wait starts so lifecycle hooks can target that request.
pub fn run_pretool_hook(payload: &Value) -> Result<Value> {
    let binding = state::prepare_request_binding(payload)?;
    Ok(json!({"requestBinding": binding}))
}

// A host Interrupt releases this session-and-turn binding while leaving the Watcher session intact.
pub fn run_interrupt_hook(payload: &Value) -> Result<Value> {
    Ok(json!({
        "cancelled": state::interrupt_request_binding(payload)?,
    }))
}

// New user input releases the active wait binding while leaving its durable Watcher session intact.
pub fn run_user_prompt_submit_hook(payload: &Value) -> Result<Value> {
    Ok(json!({
        "cancelled": state::request_binding::cancel_request_binding_for_user_prompt(payload)?,
    }))
}
