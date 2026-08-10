# Supported Topic Migration

## Scope

Migrate only an existing supported topic root. MUST preserve existing `raw/`, `wiki/`, `_index.md`, and `log.md`. MUST NOT delete, overwrite, or rename existing topic data. Unsupported material remains untouched and is not evidence for compiled articles.

## Procedure

1. MUST read the complete existing topic tree, including frontmatter, indexes,
   raw sources, and the current log.
2. MUST validate every referenced provenance and freshness input before any log
   or derived write. MUST preserve every complete relative `sources:` scalar
   exactly. If a source chain is missing, broken, weak, drifted, or
   contradictory, MUST stop, MUST report the provenance gap, and MUST leave the
   entire topic tree unchanged. If freshness data is missing or malformed, MUST
   stop, MUST report the freshness gap, and MUST leave the entire topic tree
   unchanged. A valid future date MUST receive zero freshness credit and MUST
   be reported before any derived write, but it does not invalidate provenance.
3. Only after the preflight passes, MUST append one migration entry to `log.md`
   with the topic root, source paths, index state, byte accounting, and
   freshness result.
4. MUST keep each existing raw file byte-for-byte. If a source changed, MUST
   ingest a new immutable revision instead of modifying historical raw material.
5. MUST add missing derived frontmatter only when its value is already present
   in supported data. MUST rebuild indexes from frontmatter after the
   source-of-truth Markdown is valid.
6. MUST recompile only the affected articles, then query through the normal
   master-index, category-index, and article path.

## Completion checks

MUST prove that prior raw files remain unchanged, every migrated source-backed
article resolves its source chain, and the normal query limits still hold. A
future or malformed date receives no freshness credit. If any check fails,
MUST leave existing data intact and report the gap.
