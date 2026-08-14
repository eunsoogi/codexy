# Child owner reuse

Before creating a new child Codex app thread, orchestration MUST check the
ledger and current issue/PR state for an existing issue/PR owner thread. It
MUST treat that thread as the existing owner and reuse it when present.

If that owner is usable, orchestration MUST reuse or continue it instead of
creating a duplicate owner.
