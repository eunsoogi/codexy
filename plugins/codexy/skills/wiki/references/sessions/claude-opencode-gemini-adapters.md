## Claude/OpenCode/Gemini Adapters

For Claude Code, MUST use command hooks on `SessionStart`, `UserPromptSubmit`,
`PostToolUse`, `PostToolBatch`, `PreCompact`, `PostCompact`, `Stop`, and
`SessionEnd` where available. MUST keep `SessionEnd` especially short or detached.

For OpenCode, MUST use plugin events such as session/message/tool execution and the
compaction hook surface. For Gemini CLI, MUST use `AfterTool`, `PreCompress`,
`AfterAgent`, `SessionStart`, and related hook events. All adapters normalize to
the same `.sessions/` files.

## Rehydration

Rehydration is a compact context block, not a bulk transcript paste:

```text
llm-wiki session context: review these distilled session digests before continuing.
- codex:abc123 — Automated checkpoint summary. Digest: /abs/path/HUB/.sessions/digests/2026/06/codex-abc123.md
MUST NOT treat session digests as canonical topic knowledge until promoted into a topic wiki.
```

Soft rehydration can be injected on `SessionStart` or `UserPromptSubmit` in
balanced/aggressive modes. Manual rehydration MUST be available with:

```bash
llm-wiki-session rehydrate --cwd "$PWD"
llm-wiki-session rehydrate --session-id codex:abc123
llm-wiki-session rehydrate --topic meta-llm-wiki
```

Strict forced continuation is not the default. If implemented later, it MUST be
opt-in and guard against loops with per-turn counters and stop-hook-active flags.

## Feedback Candidates

User-prompt hooks may create feedback candidates under `.sessions/feedback/`
when the user gives a correction, preference, explicit approval, or plan
acceptance. Generic acknowledgements such as `ok`, `thanks`, and `cool` are
ignored by default. Review candidates with:

```bash
llm-wiki-session feedback list --unpromoted
llm-wiki-session feedback show fb-abc123
```

Promote selected feedback with:

```bash
llm-wiki-session feedback promote fb-abc123 --topic meta-llm-wiki
```

Feedback promotion writes a distilled raw note under the target topic and logs a
`feedback` entry. See [feedback.md](../feedback.md) for taxonomy and policies.

## Promotion

Session digest promotion is explicit:

```bash
llm-wiki-session promote codex:abc123 --topic meta-llm-wiki
```

Promotion writes a topic raw note:

```text
HUB/topics/<topic>/raw/notes/YYYY-MM-DD-session-<harness>-<session>.md
```

The raw note links back to the digest and copies only the distilled digest body,
not the raw transcript. MUST append the topic `log.md` entry. Compilation into
`wiki/` articles remains a separate, explicit wiki compile/update step.

## Privacy Defaults

- Redact obvious token/password/authorization fields in event previews.
- Store transcript pointers and hashes, not transcript bodies.
- MUST keep raw transcript archiving disabled unless the user explicitly opts in.
- MUST NOT auto-promote session material or feedback candidates into topic wikis.
- MUST NOT let hook failures block normal agent work; fail closed to no capture,
  not to broken tool execution.
- To opt out, MUST run `llm-wiki-session disable` or ask `@wiki session disable`; this writes `enabled: false`.
