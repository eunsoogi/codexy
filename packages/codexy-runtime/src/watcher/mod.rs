mod io;
mod lock;
mod state;
pub mod tools;
mod tools_schema;

pub use tools::{call_tool, call_tool_with_cancellation};
pub use tools_schema::tools;

#[must_use]
pub fn server_name() -> &'static str {
    "codexy-watcher"
}
