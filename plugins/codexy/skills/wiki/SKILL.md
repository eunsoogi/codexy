---
name: wiki
description: Use only for explicit $wiki or topic-wiki requests.
---

# Wiki

MUST select only for explicit wiki intent, never for ordinary search, README
summary, planning, session memory, or unrelated research.

## Route the request

Require exactly one topic root from the explicit request; otherwise ask for it
and stop before repository reads or writes.

- For `init`, `ingest`, `compile`, `query`, `refresh`, or provenance/freshness
  verification, MUST read [Minimal Contract](references/minimal-contract.md),
  the sole owner of core operative rules.
- For `migration`, MUST read [Migration](references/migration.md), the sole
  procedure owner, and use the minimal contract for shared semantics.

These mappings are exclusive; this file owns only selection, topic routing, and
safety.

## Safety

MUST NOT store secrets, credentials, private logs, or machine-specific paths.
