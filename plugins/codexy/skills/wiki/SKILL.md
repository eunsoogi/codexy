---
name: wiki
description: A bounded, source-backed topic-wiki workflow for initialization, ingestion, compilation, cited queries, source refresh, and provenance or freshness verification.
---

# LLM Wiki

This repository-owned skill provides a compact, topic-scoped LLM memory loop.
The active topic is the bounded context; MUST NOT inspect other topics unless
the user explicitly expands scope. MUST keep user prompts and tool metadata
outside the context budget.

[Minimal Contract](references/minimal-contract.md) is the normative source for
workflow dispositions, provenance, freshness, and measurable limits.

## Core workflow

MUST use the core path `init → ingest → compile → query → refresh`.

### Init

MUST create or confirm one topic root with `raw/`, `wiki/`, `_index.md`, `log.md`,
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
articles require `compiled-from: conversation`. MUST record `updated`,
`verified`, `volatility`, and confidence. MUST compile incrementally after
`Last compiled`; MUST make a full pass explicit, then rebuild stale indexes
best-effort.

### Query

MUST read the active topic master index, a relevant category index, and only
matched articles. MUST stale-check an index before trusting it. MUST cite local
articles, report a knowledge gap instead of inferring one, and report sibling
overlap without merging sibling content. A normal query reads at most three
indexes and eight articles, with at most 4,000 UTF-8 bytes per loaded file and
48,000 total UTF-8 bytes including frontmatter. If more is needed, MUST state
why and obtain the user's explicit broader-scope intent.

### Refresh and verification

MUST compare fetchable sources against recorded provenance. A change creates a
new raw revision and marks affected knowledge for recompilation; an unchanged
source stays unchanged. MUST inspect source chains, freshness, and index
staleness. MUST report broken, weak, drifted, contradictory, missing,
malformed, or future metadata rather than hiding it. `lint` is the verification
step for this core workflow.

## Merged work

MUST use the core steps above for bounded batch ingestion, trust inspection,
evidence acquisition, derivative writing, assessment, correction, and explicit
promotion of a supported learning through the raw boundary. These activities
are not separate commands or contexts. They MUST preserve the same provenance,
log, freshness, and bounded-context rules.

## Migration

For existing supported topic data, MUST read [Migration](references/migration.md)
before changing derived files. Migration is additive and fail-closed: it
preserves source history and MUST NOT turn missing provenance into a fact.

## Safety

MUST append one operation entry to `log.md` for every write. MUST NOT store
secrets, credentials, private logs, or machine-specific paths. MUST NOT let
archived material or non-source operational records become article evidence.
