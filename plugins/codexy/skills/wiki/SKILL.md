---
name: wiki
description: Use for natural-language requests to build or operate one bounded, source-backed topic knowledge base; not for ordinary repository search, README summary, planning, session memory, or unrelated research.
---

# Wiki

MUST select for a natural-language request to build or operate one bounded,
source-backed topic knowledge base; explicit `$wiki` invocation remains
supported.

MUST NOT select for ordinary repository search, README summary, planning,
session memory, or unrelated research.

## Route the request

MUST require exactly one topic root from the explicit request; otherwise MUST
ask for it and MUST stop before repository reads or writes.

- For `init`, `ingest`, `compile`, `query`, `refresh`, or provenance/freshness
  verification, MUST read [Minimal Contract](references/minimal-contract.md),
  the sole owner of core operative rules.
- For `migration`, MUST read [Migration](references/migration.md), the sole
  procedure owner, and use the minimal contract for shared semantics.

These mappings are exclusive; this file owns only selection, topic routing, and
safety.

## Safety

MUST NOT store secrets, credentials, private logs, or machine-specific paths.
