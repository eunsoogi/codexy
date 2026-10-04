MAX_INPUT_BYTES = 1024 * 1024
EVENTS = ("PreToolUse", "Interrupt", "UserPromptSubmit")
UNSUPPORTED_INTERPRETER_EXIT = 125


def handle_input_event(event, payload, invoke):
    if event == "Interrupt":
        # Preserve the complete host payload so runtime cancellation can match its session and turn.
        invoke("--hook-interrupt", payload)
        return
    # Prompt text is irrelevant to cancellation; scope the notification to the originating session.
    session_id = payload.get("session_id")
    if isinstance(session_id, str) and session_id:
        invoke("--hook-user-prompt-submit", {"session_id": session_id})


HOOK_API = (MAX_INPUT_BYTES, EVENTS, UNSUPPORTED_INTERPRETER_EXIT, handle_input_event)
