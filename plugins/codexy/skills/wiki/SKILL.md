---
name: wiki
description: Use only when the user explicitly invokes $wiki or explicitly asks for a bounded topic-wiki workflow, including initialization, ingestion, compilation, cited query, refresh, verification, or migration.
---

# Wiki

Use this skill only for explicit wiki intent. Ordinary repository search, README
summarization, planning, session memory, and unrelated research MUST NOT select
this skill.

## Resolve the topic

Resolve exactly one explicit topic root before reading or writing. If it is
missing, request it and perform no broader read or write. The topic root is the
only active context unless the user explicitly approves a broader scope.

## Route the request

- For `init`, create or confirm only `raw/`, `wiki/`, `_index.md`, `log.md`, and
  `config.md` under the topic root.
- For `ingest`, add one immutable raw revision with source identity, ingested
  date, tags, and summary. A changed source creates another revision; it MUST
  NOT overwrite history.
- For `compile`, `query`, `refresh`, or provenance/freshness verification, read
  and follow [Minimal Contract](references/minimal-contract.md), the sole owner
  of workflow, provenance, freshness, and bounded-read rules.
- For `migration`, read [Migration](references/migration.md), the sole owner of
  additive fail-closed migration procedure. Apply the minimal contract when
  verifying the migrated result or answering a query.

## Write safety

Append exactly one operation entry to `log.md` for every write. Preserve raw
history. MUST NOT store secrets, credentials, private logs, or machine-specific
paths. Inventory, sessions, projects, archives, and operational records MUST NOT
become factual wiki sources.
