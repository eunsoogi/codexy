use super::super::{McpClient, Path, binding_cancelled, run_hook, watcher_state};
use super::wait_is_active;
use serde_json::json;

pub(super) fn open_after_live_binding(
    plugin: &Path,
    cache: &Path,
    state: &Path,
    waiter: &mut McpClient,
    binding: &str,
    first_session: &str,
    first_parent: &str,
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let mut second_setup = watcher_state::watcher_client(state)?;
    watcher_state::initialize(&mut second_setup)?;
    let second = watcher_state::open_session(&mut second_setup, "second", 2);
    drop(second_setup);
    let second = match second {
        Ok(session) => session,
        Err(error) => {
            // Release the real waiter before reporting the regression's expected RED result.
            let cleanup = run_hook(
                plugin,
                cache,
                state,
                Path::new(""),
                "linux-x86_64",
                "UserPromptSubmit",
                json!({"hook_event_name":"UserPromptSubmit","session_id":"main-session",
                    "turn_id":"cleanup","prompt":"finish failed session open"}),
            );
            if cleanup.as_ref().is_ok_and(|output| output.status.success()) {
                let response = waiter.read_frame()?;
                assert_eq!(
                    watcher_state::tool_payload(&response)?["status"],
                    "cancelled"
                );
            } else {
                let _ = waiter.child.kill();
            }
            return Err(format!(
                "opening an independent session with a live request binding failed: {error}"
            )
            .into());
        }
    };
    let binding_path = state
        .join("codexy-watcher/.request-bindings")
        .join(format!("{binding}.json"));
    assert!(
        binding_path.is_file(),
        "opening the second session removed the first binding"
    );
    assert!(!binding_cancelled(state, binding));
    assert!(wait_is_active(state, first_session, first_parent)?);
    Ok(second)
}
