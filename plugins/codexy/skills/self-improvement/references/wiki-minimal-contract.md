# Minimal Wiki Contract

For one topic root, this solely owns workflow, provenance, freshness, bounds,
raw history, and per-write logging. Markdown/frontmatter is authoritative;
indexes are caches. The path is `init → ingest → compile → query → refresh`.

## Workflow

- **Init:** MUST create or confirm only `raw/`, `wiki/`, `_index.md`, `log.md`,
  and `config.md` under the root.
- **Ingest:** MUST add an immutable `raw/` revision with title, source identity,
  type, ingested date, tags, and summary. MUST preserve an available source
  revision or hash, canonical URL, and item provenance.
- **Compile:** MUST synthesize raw evidence. Source-backed articles MUST use
  root-relative `sources:`; conversation-only articles MUST use
  `compiled-from: conversation`. MUST record `updated`, `verified`,
  `volatility`, and confidence. MUST compile after `Last compiled` by default;
  full passes MUST be explicit. MUST rebuild stale indexes from frontmatter.
- **Query:** MUST read `_index.md`, one category index, then only matches; MUST
  stale-check first. MUST cite local articles and resolved sources or MUST
  report a gap. Limits are three indexes, eight articles, 4,000 UTF-8 bytes per
  loaded file, and 48,000 total including frontmatter. MUST state the reason and
  MUST obtain explicit broader intent before exceeding any limit.
- **Refresh:** MUST compare fetchable sources with recorded provenance.
  Unchanged sources stay untouched; changed content creates a new raw revision
  and a recompile requirement.

## Provenance and freshness

- MUST resolve every `sources:` value as one complete root-relative path; MUST
  NOT split whitespace. MUST report missing, broken, weak, drifted, or
  contradictory chains.
- Source-backed freshness is 0–100: source, verification, compilation, and chain
  integrity contribute 0–25 each. MUST use UTC days. Missing, malformed, or
  future `ingested`, `verified`, `updated`, or `created` contributes zero and
  MUST be reported; future age is never zero. Compilation uses valid `updated`,
  then `created`. Source age averages valid resolved sources; unresolved sources
  remain in the chain denominator. Unknown volatility is `warm`.
- Half-lives are hot 30, warm 90, cold 365 days. Each recency component is
  `25 * 0.5^(age_days / half_life_days)`; chain integrity is
  `25 * resolved / total`. MUST round .5 upward and MUST clamp to 0–100.
  `compiled-from: conversation` doubles verification plus compilation; mixed
  uses the source-backed formula. MUST flag scores below `freshness_threshold`
  (default 70).

## Guarantees

- MUST append exactly one operation entry to `log.md` for every write.
- The resolved root is the bound. Sibling content requires explicit expanded
  scope. Inventory, datasets, projects, sessions, archives, and operational
  records MUST NOT become facts.
- A missing root, broken provenance, malformed required metadata, or unapproved
  broader read MUST fail closed and remain visible.
