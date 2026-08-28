# Minimal Wiki Contract

This is the sole normative workflow, provenance, freshness, and bounded-query
contract for one explicitly resolved topic root. Markdown and frontmatter are
source of truth; indexes are derived caches. The path is
`init → ingest → compile → query → refresh`.

## Workflow

### Init and ingest

- Init MUST create or confirm only `raw/`, `wiki/`, `_index.md`, `log.md`, and
  `config.md` under the explicit topic root.
- Ingest MUST write accepted material as a new immutable `raw/` revision with
  title, source identity, type, ingested date, tags, and summary. Preserve an
  available source revision or content hash, canonical URL, and per-item
  provenance. Changed content MUST create a new revision.

### Compile

- Compile MUST synthesize from raw evidence rather than copy it. A source-backed
  article MUST contain non-empty wiki-root-relative `sources:` entries; a
  conversation-only article MUST declare `compiled-from: conversation`.
- Record `updated`, `verified`, `volatility`, and confidence. Compile sources
  newer than `Last compiled` by default; a full pass MUST be explicit. Rebuild
  stale indexes from frontmatter after source-of-truth writes.

### Query

- Query MUST start with `_index.md`, then one relevant category index, then only
  matched articles. Stale-check indexes before trusting them. Cite local
  articles and their resolvable source chain; report a gap instead of inferring.
- Normal reads allow at most three indexes, eight articles, 4,000 UTF-8 bytes
  per index or article, and 48,000 total UTF-8 bytes including frontmatter. If
  more is needed, state the bound and reason, then obtain explicit broader-scope
  intent before loading more.

### Refresh and provenance

- Refresh MUST compare each fetchable source with recorded provenance. Leave an
  unchanged source untouched; write changed content as a new raw revision and
  record the recompile requirement.
- Resolve every `sources:` scalar exactly as a complete wiki-root-relative path;
  MUST NOT split paths on whitespace. Report missing, broken, weak, drifted, or
  contradictory chains. A source-backed article with no resolvable source is a
  provenance gap, never a clean result.

## Freshness

- Every source-backed article MUST provide `volatility`, `verified`, and a
  compilation date. Score 0–100 from source, verification, and compilation
  recency plus source-chain integrity, each worth 0–25.
- Use UTC calendar days. Missing, malformed, or future `verified`, `updated`,
  `created`, or raw `ingested` metadata contributes zero to its component and
  MUST be reported; future age MUST NOT be converted to zero. Use `updated` for
  compilation recency when valid, otherwise valid `created`, otherwise zero.
- `source_age` is the average age of resolvable sources with valid non-future
  `ingested` dates; no valid source dates means zero source freshness.
  Unresolved sources remain in the source-chain denominator. Unknown
  `volatility` defaults to `warm`.
- A `compiled-from: conversation` article MUST omit raw-source and source-chain
  components and double the verification-plus-compilation subtotal. Mixed
  articles use the source-backed formula. Flag scores below `config.md`'s
  `freshness_threshold`, default 70.

```text
freshness.hot_half_life_days = 30
freshness.warm_half_life_days = 90
freshness.cold_half_life_days = 365
freshness.decay = 25 * 0.5^(age_days / half_life_days)
freshness.source_chain = 25 * resolvable_sources / total_sources
freshness.score = round_half_up(decay(source_age) + decay(verification_age) + decay(compilation_age) + source_chain)
freshness.future_date = 0
freshness.conversation = min(100, 2 * (verification + compilation))
```

For valid non-future dates, `age_days = today_utc - recorded_utc_day`.
`round_half_up` rounds .5 upward and clamps the final score to 0–100. Every
verification MUST expose gaps and malformed or future metadata rather than
rewriting or hiding them.

## Shared guarantees

- Append exactly one operation entry to `log.md` for every write.
- The resolved topic is the bound. Sibling content requires explicit expanded
  scope. Inventory, datasets, projects, sessions, archives, and operational
  records MUST NOT become factual evidence.
- Broken provenance, malformed required metadata, a missing topic root, or an
  unapproved broader read MUST fail closed.
