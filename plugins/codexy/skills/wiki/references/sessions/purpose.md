# Session Context Capture

MUST use this reference for `session`, `session capture`, `rehydrate`, "look at the
last session", automated hook capture, and promotion of session learnings into a
topic wiki.

## Purpose

Session capture preserves agent-session context without polluting curated topic
wikis. The session layer records redacted harness metadata and markdown digests
under the hub-level operational directory `HUB/.sessions/`. Topic wikis receive
session knowledge only through explicit promotion into `raw/notes/`.

This layer is separate from existing research-session files:

| Layer | Path | Meaning |
|---|---|---|
| Research crash state | `.research-session.json`, `.thesis-session.json` | In-progress wiki research/thesis runs |
| Durable research provenance | `.session-events.jsonl`, `.session-checkpoint.json` | Replayable provenance for wiki workflows |
| Harness sessions | `HUB/.sessions/` or `.wiki/.sessions/` | Cross-runtime Codex/Claude/OpenCode/Gemini session context |

## Storage Layout

```text
HUB/.sessions/
├── config.json
├── registry.jsonl
├── state/<harness>/<session_id>.json
├── queue/YYYY-MM-DD.jsonl
├── digests/YYYY/MM/<harness>-<session_id>.md
├── feedback/
│   ├── candidates.jsonl
│   └── status.json
└── indexes/
    ├── by-cwd.json
    ├── by-topic.json
    ├── feedback.json
    └── sessions.json
```

MUST use the same layout under `.wiki/.sessions/` for local project wikis.

- `config.json` controls automation and rehydration. If it is absent, capture defaults to `balanced`; `enabled: false` is the opt-out switch.
- `registry.jsonl` is append-only lifecycle metadata such as `session_seen` and
  `digest_written`.
- `queue/*.jsonl` stores small redacted hook events. MUST NOT store full prompts,
  tool outputs, or transcript bodies by default.
- `state/` is the latest per-session machine state.
- `digests/` contains human/LLM-readable markdown checkpoints with YAML
  frontmatter.
- `feedback/` stores redacted, reviewable user-feedback candidates such as
  corrections, preferences, approvals, and plan acceptance.
- `indexes/` are derived caches rebuilt from `state/` and feedback candidates.

`.sessions/` is hidden because it is operational, cross-topic, and potentially
private. It MUST NOT be compiled as ordinary topic content. A future generated
`HUB/_sessions.md` can expose a human-friendly index, but the canonical store is
`.sessions/`.

## Config Modes

Default mode is `balanced`, even before a config file exists:

```json
{
  "schema_version": 1,
  "enabled": true,
  "mode": "balanced",
  "auto_capture": {
    "tool_events": 50,
    "pre_compact": true,
    "post_compact": true,
    "session_end": true,
    "stop": true
  },
  "rehydrate": {
    "session_start": true,
    "user_prompt": true,
    "strict": false
  },
  "raw_transcripts": false,
  "privacy": "redacted",
  "feedback": {
    "enabled": true,
    "capture_approvals": true,
    "min_confidence": "medium"
  }
}
```

Modes:

| Mode | MUST capture | Rehydrate | Scenario |
|---|---|---|---|
| `off` | none | none | Disabled / user opt-out |
| `capture-only` | automatic checkpoints | no injected context | Maximum privacy/least surprise |
| `balanced` | automatic checkpoints | SessionStart/UserPromptSubmit soft context | Recommended default |
| `aggressive` | more frequent checkpoints | soft context | Long, high-context work |

## Capture Triggers

Hooks MUST write deterministic checkpoints for:

1. every configured number of observed tool events, default 50;
2. pre-compaction and post-compaction;
3. session stop/end events;
4. manual `session capture` requests.

"Observed tool events" means events the current harness adapter exposes. Codex,
Claude, OpenCode, and Gemini have different hook coverage, so MUST NOT promise an
exact total count across all tool classes.

## Digest Schema

Session digest files are markdown with YAML frontmatter:

```yaml
title: "Session Digest: codex:abc123"
type: session-digest
schema_version: 1
harness: "codex"
native_session_id: "abc123"
llm_wiki_session_id: "codex:abc123"
cwd: "/path/to/project"
git_remote: "https://github.com/org/repo.git"
git_branch: "feature/example"
transcript_path: "/path/to/transcript.jsonl"
started_at: "2026-06-13T17:00:00Z"
last_seen_at: "2026-06-13T18:10:00Z"
tool_event_count: 50
event_count: 61
capture_trigger: "tool-count-50"
topics: []
topic_candidates: []
privacy: "redacted"
raw_transcripts: false
promoted_to: []
summary: "Automated checkpoint for codex:abc123."
```

The body MUST include session identity, summary, manual notes if any, recent
observed events, distillation notes, and open loops. It MUST NOT include full
transcript text by default.

## Hook Adapter Contract

Each runtime adapter MUST call the same deterministic helper shape:

```bash
llm-wiki-session hook --harness <codex|claude|opencode|gemini> --if-enabled
```

The command reads the harness hook JSON object from stdin and MUST:

1. MUST resolve HUB from `~/.config/llm-wiki/config.json` unless `--hub` or `--local`
   is supplied;
2. exit 0 without writing if `--if-enabled` is set and `.sessions/config.json`
   has `enabled: false`; missing config means default-on balanced capture;
3. MUST append a redacted event to `queue/YYYY-MM-DD.jsonl`;
4. MUST update `state/<harness>/<session_id>.json`;
5. MUST write a digest if a capture trigger fired;
6. optionally append a redacted feedback candidate for high-signal user turns;
7. optionally emit a short `additionalContext` block only for rehydration hooks
   when config enables it.

Hooks MUST be fast. If future versions do LLM distillation, enqueue work for a
background worker instead of doing it inline inside every tool hook.

## Codex Hook Packaging

Codex plugins can bundle hooks in `hooks/hooks.json`. Plugin-bundled hooks still
MUST require the user to review/trust them. The bundled llm-wiki Codex hook MUST
call the copied helper in the plugin root and MUST use `--if-enabled`, so users can
turn capture off with `session disable` without editing hook manifests.

Useful Codex events:

- `SessionStart` and `UserPromptSubmit` for soft rehydration;
- `PostToolUse` for observed tool counters;
- `PreCompact` and `PostCompact` for compaction checkpoints;
- `Stop` for final checkpoints.
