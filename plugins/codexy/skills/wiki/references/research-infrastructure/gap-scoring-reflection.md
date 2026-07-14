## Gap Scoring & Reflection

### Why

Between multi-round research rounds, reflect holistically on accumulated knowledge and score gaps for the next round. **Key insight from testing**: plan reflection's primary value is discovering cross-topic connections between rounds — NOT changing the research direction. Testing against a real 4-round research wiki showed the research path was already well-chosen (reflection confirmed every round's direction). But it found 5 undrawn cross-references that exist in the content but were not linked. This is the 34% improvement the literature predicts.

### Gap Scoring Formula

Each gap is scored on three dimensions (1-5 each):

| Dimension | 5 (highest) | 3 (moderate) | 1 (lowest) |
|-----------|-------------|--------------|------------|
| **Impact** | Filling this gap fundamentally changes understanding | Adds useful context | Nice-to-know but not essential |
| **Feasibility** | Likely findable with web search | May exist but hard to find | Probably requires primary research |
| **Specificity** | Well-defined, searchable question | Somewhat vague | Too broad to target effectively |

**Composite score** = Impact x Feasibility x Specificity (range: 1-125)

**Selection**: Pick top 3 gaps by composite score for the next round.

### Reflection Protocol

Between rounds, the orchestrating agent MUST, in priority order:

1. **Draw connections** between this round's findings and ALL prior rounds (not just the previous one) — this is the highest-value activity
2. **MUST update cross-references** — MUST add See Also links between articles that share concepts across rounds
3. **Re-evaluate earlier gaps** — some gaps from round 1 may now be filled or irrelevant
4. **Score remaining gaps** using the formula above
5. **Adjust research direction** — only if findings clearly indicate a shift (rare in practice)
6. **Note reflection in session registry** — MUST add `reflection_notes` to the round entry

### Example Reflection Output

```
## Round 2 Reflection

### Cross-Topic Connections Discovered
- Round 1 finding about X connects to Round 2 finding about Y
- This suggests a new gap: "How does X influence Y?"

### Gap Re-Evaluation
- Gap "A" from Round 1: now filled by Round 2 sources (remove)
- Gap "B" from Round 1: still unfilled, upgraded to high-impact (keep)
- New gap "C": emerged from Round 2 findings (add)

### Scored Gaps for Round 3
1. Gap B: Impact 5 x Feasibility 4 x Specificity 5 = 100
2. Gap C: Impact 4 x Feasibility 5 x Specificity 4 = 80
3. Gap D: Impact 3 x Feasibility 3 x Specificity 4 = 36

### Direction Shift
Research initially focused on X but findings consistently point to Y as the more important subtopic. Round 3 MUST emphasize Y.
```

---

## Session Registry

### Why

Persistent state for multi-round research and thesis sessions, enabling crash recovery and round-to-round continuity. Without this, a crashed `--min-time` session loses all round state and the user has to start over. The file is ephemeral (not committed to git, not indexed), cheap to lose (worst case: user is asked "continue or start fresh?"), but valuable to have.

### Research Session Schema (.research-session.json)

```json
{
  "session_id": "2026-04-06-143022",
  "topic": "research topic",
  "start_time": "2026-04-06T14:30:22Z",
  "min_time_budget": "2h",
  "current_round": 2,
  "rounds_completed": [
    {
      "round": 1,
      "start_time": "2026-04-06T14:30:22Z",
      "end_time": "2026-04-06T15:02:45Z",
      "sources_ingested": 5,
      "articles_compiled": 3,
      "gaps": ["gap1 description", "gap2 description"],
      "progress_score": 65,
      "reflection_notes": "Initial broad coverage complete. Gap1 is highest priority."
    }
  ],
  "cumulative_sources": 5,
  "cumulative_articles": 3,
  "status": "in_progress"
}
```

### Thesis Session Schema (.thesis-session.json)

```json
{
  "session_id": "2026-04-06-143022",
  "thesis": "claim statement",
  "current_round": 2,
  "rounds_completed": [
    {
      "round": 1,
      "evidence_for": 4,
      "evidence_against": 2,
      "verdict_direction": "partially-supported",
      "next_round_focus": "opposing"
    }
  ],
  "status": "in_progress"
}
```

### Durable Provenance Files

The session registry files above are for **live recovery**. They are not the
best long-term provenance format because they are overwritten in place and
deleted on normal completion.

Research, thesis, audit, and related long-running wiki workflows MUST maintain
two durable provenance artifacts in the wiki root in addition to session registry files:

- `.session-events.jsonl` — append-only event log
- `.session-checkpoint.json` — latest replayable summary

These files persist after normal completion and are what the audit layer uses
to classify provenance as `replayable` instead of merely `partial`.

### Event Log Schema (.session-events.jsonl)

Each line is one JSON object. MUST append only; MUST NOT rewrite prior entries.

```json
{"ts":"2026-04-29T12:00:00Z","command":"research","phase":"start","event":"research_started","session_id":"2026-04-29-120000","topic":"cerebral amyloid angiopathy","mode":"single","min_time_budget":"2h"}
{"ts":"2026-04-29T12:38:00Z","command":"research","phase":"round","event":"research_round_completed","session_id":"2026-04-29-120000","round":1,"sources_ingested":5,"articles_compiled":3,"progress_score":65}
{"ts":"2026-04-29T12:42:00Z","command":"research","phase":"reflection","event":"research_reflection_completed","session_id":"2026-04-29-120000","round":1,"top_gaps":["gap1","gap2","gap3"]}
{"ts":"2026-04-29T14:05:00Z","command":"research","phase":"finish","event":"research_completed","session_id":"2026-04-29-120000","rounds_completed":3,"cumulative_sources":14,"cumulative_articles":9}
```

Recommended fields:

| Field | Type | Purpose |
|-------|------|---------|
| `ts` | string | ISO 8601 timestamp |
| `command` | string | `research`, `audit`, `output`, `refresh`, etc. |
| `phase` | string | `start`, `round`, `reflection`, `scan`, `finish`, etc. |
| `event` | string | Stable event name |
| `session_id` | string | Correlates all entries from one run |
| `topic` / `thesis` / `scope` | string | Human-readable target |
| `round` | number | Research/thesis round when applicable |
| `sources_ingested` | number | Per-round or cumulative count when relevant |
| `articles_compiled` | number | Per-round or cumulative count when relevant |
| `progress_score` | number | Round quality signal when relevant |
| `artifacts` | array | Paths written in that step |
| `notes` | string | Short freeform summary, optional |

### Checkpoint Schema (.session-checkpoint.json)

The checkpoint is the latest compact summary of the most recent important run.
Rewrite atomically after each meaningful milestone.

```json
{
  "updated_at": "2026-04-29T14:05:00Z",
  "command": "research",
  "session_id": "2026-04-29-120000",
  "status": "completed",
  "topic": "cerebral amyloid angiopathy",
  "current_round": 3,
  "summary": {
    "cumulative_sources": 14,
    "cumulative_articles": 9,
    "last_progress_score": 82,
    "top_open_gaps": ["gap4", "gap5"]
  },
  "artifacts": [
    {
      "path": "output/2026-04-29-caa-summary.md",
      "sha256": "abc123..."
    }
  ]
}
```

Recommended fields:

| Field | Type | Purpose |
|-------|------|---------|
| `updated_at` | string | ISO 8601 timestamp |
| `command` | string | Command that owns the checkpoint |
| `session_id` | string | Correlates with event log and ephemeral session file |
| `status` | string | `in_progress`, `completed`, `interrupted`, `failed` |
| `topic` / `thesis` / `scope` | string | Human-readable target |
| `current_round` | number | Most recent completed round, when applicable |
| `summary` | object | Compact state for resume briefings |
| `artifacts` | array | Written artifact paths and hashes, when available |
