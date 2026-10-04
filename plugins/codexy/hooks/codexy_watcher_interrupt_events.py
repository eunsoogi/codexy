MAX_INPUT_BYTES = 1024 * 1024
EVENTS = ("PreToolUse", "Interrupt", "UserPromptSubmit")
UNSUPPORTED_INTERPRETER_EXIT = 125


def handle_input_event(event, payload, invoke):
    if event == "Interrupt":
        invoke("--hook-interrupt", payload)
        return
    session_id = payload.get("session_id")
    if isinstance(session_id, str) and session_id:
        invoke("--hook-user-prompt-submit", {"session_id": session_id})


HOOK_API = (MAX_INPUT_BYTES, EVENTS, UNSUPPORTED_INTERPRETER_EXIT, handle_input_event)
