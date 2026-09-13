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

pub fn run_pretool_hook(payload: &Value) -> Result<Value> {
    let binding = state::prepare_request_binding(payload)?;
    Ok(json!({"requestBinding": binding}))
}

pub fn run_interrupt_hook(payload: &Value) -> Result<Value> {
    Ok(json!({
        "cancelled": state::interrupt_request_binding(payload)?,
    }))
}
