---
name: wiki
description: Use for a bounded, source-backed topic wiki: initialize it, ingest material, compile articles, query cited knowledge, refresh sources, or verify provenance and freshness.
---

# LLM Wiki

Use this repository-owned skill only for a compact, topic-scoped LLM memory loop.
The active topic is the bounded context; do not inspect other topics unless the
user explicitly expands scope. Keep user prompts and tool metadata outside the
context budget.

[Minimal Contract](references/minimal-contract.md) is the normative source for
workflow dispositions, provenance, freshness, and measurable limits.

## Core workflow

MUST use the core path `init → ingest → compile → query → refresh`.

### Init

Create or confirm one topic root with `raw/`, `wiki/`, `_index.md`, `log.md`,
and `config.md`. `config.md` MAY set `freshness_threshold`; its default is 70.
Markdown frontmatter is the source of truth and indexes are derived caches.

### Ingest

MUST write accepted material as a new immutable `raw/` source with title,
source, type, ingested date, tags, and summary. A changed source MUST create a
new raw revision, preserving upstream identity, revision or content hash when
available, canonical URL, and per-item provenance. A bounded source batch uses
this same rule; it MUST NOT overwrite an earlier raw revision.

### Compile

MUST synthesize articles from raw sources rather than copy them. Source-backed
articles require non-empty wiki-root-relative `sources:`; conversation-only
articles require `compiled-from: conversation`. Record `updated`, `verified`,
`volatility`, and confidence. Compile incrementally after `Last compiled`;
make a full pass explicit, then rebuild stale indexes best-effort.

### Query

MUST read the active topic master index, a relevant category index, and only
matched articles. Stale-check an index before trusting it. Cite local articles,
report a knowledge gap instead of inferring one, and report sibling overlap
without merging sibling content. A normal query reads at most three indexes and
eight articles, with at most 4,000 UTF-8 bytes per loaded file and 48,000 total
UTF-8 bytes including frontmatter. If more is needed, state why and obtain the
user's explicit broader-scope intent.

### Refresh and verification

MUST compare fetchable sources against recorded provenance. A change creates a
new raw revision and marks affected knowledge for recompilation; an unchanged
source stays unchanged. Inspect source chains, freshness, and index staleness.
Report broken, weak, drifted, contradictory, missing, malformed, or future
metadata rather than hiding it. `lint` is the verification step for this core
workflow.

## Merged work

Use the core steps above for bounded batch ingestion, trust inspection,
evidence acquisition, derivative writing, assessment, correction, and explicit
promotion of a supported learning through the raw boundary. These are not
separate commands or contexts. They MUST preserve the same provenance, log,
freshness, and bounded-context rules.

## Migration

For existing supported topic data, read [Migration](references/migration.md)
before changing derived files. Migration is additive and fail-closed: it
preserves source history and never turns missing provenance into a fact.

## Safety

MUST append one operation entry to `log.md` for every write. MUST NOT store
secrets, credentials, private logs, or machine-specific paths. MUST NOT let
archived material or non-source operational records become article evidence.
