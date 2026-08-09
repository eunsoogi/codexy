# Minimal LLM Wiki Contract

## Purpose and boundary

This is the smallest durable contract for an LLM-compiled wiki. It defines the
knowledge path and its measurable guarantees while preserving current workflow
implementation. A `Remove` disposition labels a workflow outside this minimal
contract. Inventory policy is owned by #543; MCP and LSP surfaces are owned by #544.

The normative path is `init → ingest → compile → query → refresh`. `lint`,
`librarian`, and `audit` verify the path; other workflows remain compatible
extensions only when they preserve these rules.

## Essential contract

### Ingest

- Ingest MUST write accepted material as an immutable `raw/` source with title,
  source, type, ingested date, tags, and summary.
- A collection ingest MUST also preserve its upstream identity, revision or
  content hash when available, canonical URL, and per-item provenance.
- A changed upstream item MUST create a new raw revision; it MUST NOT overwrite
  the prior source. Candidate tracking and dataset policy are not factual input
  to compilation.

### Compile

- Compile MUST synthesize articles from `raw/` sources and MUST NOT copy them. A
  source-backed article MUST contain non-empty, wiki-root-relative `sources:`;
  a conversation-only article MUST declare `compiled-from: conversation`.
- Incremental compile MUST select sources newer than `Last compiled`; full
  recompilation MUST be explicit. It MUST update derived indexes best-effort after
  writing source-of-truth Markdown and frontmatter.
- Compilation MUST record `updated`, `verified`, `volatility`, and confidence,
  and MUST NOT use inventory records or archived content as article facts.

### Query

- Query MUST start from the resolved topic wiki's master `_index.md`, then a
  relevant category index, then only matched articles. It MUST stale-check each
  index before trusting it and rebuild a stale cache from frontmatter.
- A query MUST answer from cited local articles, state a gap rather than infer
  missing knowledge, and inspect sibling indexes only for reported overlap.
  It MUST NOT merge sibling content into the active topic implicitly.

### Refresh and provenance

- Refresh MUST compare each fetchable source with its recorded provenance. A
  changed source becomes a new immutable raw revision; unchanged sources remain
  unchanged. Refresh MUST mark affected derived knowledge for recompilation or
  MUST report why it remains current.
- Every source-backed article and generated output MUST resolve its `sources:`
  chain exactly. Resolution MUST preserve complete YAML scalar paths and MUST NOT
  split filenames on whitespace. Missing, broken, weak, drifted, or contradictory
  chains MUST be reported rather than hidden.

### Bounded context

- The resolved topic wiki is the active bounded context. Its raw files and
  articles are authoritative for the request; sibling wikis contribute indexes
  only unless the user explicitly expands scope.
- Archived topics are excluded from normal ingest, compile, query, refresh, and
  audit work. Inventory, datasets, projects, and session artifacts can support
  operations but MUST NOT silently become article evidence.

## Measurable criteria

### Context efficiency

- A normal query MUST read no more than three index files and no more than eight
  matched article files. If it needs more, it MUST state the reason and obtain
  explicit broader-scope intent.
- Context size is the UTF-8 byte length of every loaded index and article,
  including frontmatter. The total excludes the user prompt and tool metadata.
  A normal query MUST satisfy every machine-checkable limit below; broader scope
  MUST record the requested limit and the user's explicit approval.
- Compile uses the `Last compiled` boundary by default; a full pass MUST be
  explicit. Index staleness is measured by Markdown-file count versus index-row
  count before an index guides a read.

### Traceability

- 100% of source-backed articles MUST have at least one resolvable `sources:`
  entry. 100% of conversation-only articles MUST state `compiled-from:
  conversation`.
- 100% of generated outputs with factual dependencies MUST have resolvable
  `sources:`. Audits MUST classify a broken or missing chain as `provenance-gap`,
  rather than clean.
- Every write MUST append one operation entry to `log.md`; raw revisions preserve the
  source identity and immutable historical record needed to replay provenance.

### Freshness

- Every source-backed article MUST provide `volatility`, `verified`, and
  `updated`. Its freshness score is 0–100 from source freshness, verification
  recency, compilation recency, and source-chain integrity; each dimension is
  worth 0–25.
- Dates use their UTC calendar day. Missing, malformed, or future `verified`,
  `updated`, or raw `ingested` dates MUST contribute zero to that component and
  MUST NOT be converted to an age of zero. For a multi-source article,
  `source_age` is the average age among its resolvable sources with valid,
  non-future `ingested` dates; if none exist, source freshness is zero. Unknown
  volatility defaults to `warm`. For source-backed articles, unresolved sources
  count in the source-chain denominator but not its numerator.
- A `compiled-from: conversation` article MUST omit raw-source and source-chain
  components, score verification and compilation as below, then double their
  subtotal. A `compiled-from: mixed` article MUST use the source-backed formula.
- Articles below `freshness.threshold` MUST be flagged for refresh. Refresh MUST
  report either an unchanged comparison or a new raw revision and the resulting
  recompile requirement; it MUST NOT overwrite raw history.

### Machine-checkable limits

```text
query.max_index_files = 3
query.max_article_files = 8
query.max_index_file_bytes = 4000
query.max_article_file_bytes = 4000
query.max_total_bytes = 48000
freshness.threshold = 70
freshness.hot_half_life_days = 30
freshness.warm_half_life_days = 90
freshness.cold_half_life_days = 365
freshness.decay = 25 * 0.5^(age_days / half_life_days)
freshness.source_age = average(age_days across resolvable sources)
freshness.source_chain = 25 * resolvable_sources / total_sources
freshness.score = round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)
freshness.future_date = 0
freshness.conversation = min(100, 2 * (verification + compilation))
```

For valid non-future values, `age_days` is `today_utc - recorded_utc_day`.
`round_half_up` rounds to the nearest integer with .5 rounded upward, then
clamps the final score to 0–100. A source-backed article with no valid source
entries is a traceability failure and MUST NOT receive a clean freshness result.

## Current workflow disposition

This table is the closed current workflow inventory for this skill. Every row
MUST occur exactly once with one `Keep`, `Merge`, or `Remove` disposition.
Before a new current workflow can claim contract coverage, it MUST add one row.

| Current workflow | Disposition | Contract role |
| --- | --- | --- |
| `init` | Keep | Establishes a topic root and its source-of-truth/index layout. |
| `ingest` | Keep | Admits immutable, provenance-bearing source material. |
| `ingest-collection` | Merge | Uses ingest's immutable/provenance contract for bounded source collections. |
| `collect` | Remove | Candidate discovery is outside the minimal factual pipeline; it may hand off to inventory. |
| `compile` | Keep | Produces synthesized, provenance-linked knowledge from raw sources. |
| `query` | Keep | Reads bounded indexes and cited articles, reporting knowledge gaps. |
| `refresh` | Keep | Rechecks fetchable provenance and creates new raw revisions when changed. |
| `lint` | Keep | Checks structural and freshness-rule conformance. |
| `librarian` | Merge | Maintains the wiki layer under refresh, provenance, and freshness criteria. |
| `audit` | Merge | Performs umbrella trust inspection using provenance and freshness results. |
| `research` | Merge | Acquires evidence through ingest before it can affect compiled knowledge. |
| `output` | Merge | Generates derivative artifacts with resolvable source chains. |
| `plan` | Remove | Planning is orchestration, not a minimal knowledge-path operation. |
| `project` | Remove | Project organization is optional output management. |
| `inventory` | Remove | Candidate/decision policy is adjacent operational state, owned separately from this contract. |
| `dataset` | Remove | Registry management is an optional boundary for large or mutable data. |
| `archive` | Remove | Lifecycle preservation is not part of the active knowledge path. |
| `ll` | Remove | Compatibility/status shorthand is not a contract capability. |
| `assess` | Merge | Assessment consumes the same bounded, traceable, fresh evidence as query and audit. |
| `retract` | Merge | Retracts or repairs knowledge through the same provenance and log rules. |
| `thesis` | Merge | Research variant that MUST ingest evidence before compiled use. |
| `status` | Remove | Status inspection is not a minimal knowledge-path operation. |
| `session` | Remove | Operational session routing is not compiled topic knowledge. |
| `session-capture` | Remove | Redacted operational memory stays outside topic facts by default. |
| `rehydrate` | Remove | Session-context recovery is operational memory, not compiled knowledge. |
| `session-promote` | Merge | Explicit promotion admits a session learning through the raw-note boundary. |
| `feedback` | Remove | Feedback routing is candidate-memory operation, not a factual source. |
| `feedback-capture` | Remove | Candidate feedback remains reviewable operational memory. |
| `feedback-promote` | Merge | Explicit promotion writes a bounded raw note with provenance. |

Cross-cutting index maintenance and append-only `log.md` entries are kept as
required behavior inside every applicable core write or read, rather than as
independent commands. Natural-language and `@wiki` invocation remain the
interface; `/wiki:*` examples are shorthand, not registered Codex commands.
