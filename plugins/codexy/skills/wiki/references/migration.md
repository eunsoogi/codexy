# Topic Migration

Migrate only one explicit existing topic root. Migration is additive and MUST
NOT delete, overwrite, or rename existing topic data.

1. Read the complete topic tree and snapshot its bytes.
2. Before any write, validate every provenance path and required freshness input
   using [Minimal Contract](minimal-contract.md). Preserve each complete
   relative `sources:` scalar. Missing, broken, weak, drifted, contradictory, or
   malformed input MUST stop migration, report the gap, and leave all topic
   bytes unchanged. Report future dates with zero affected freshness credit.
3. Keep every existing raw file byte-for-byte. A changed source becomes a new
   immutable raw revision.
4. Stage derived changes and the single completion log entry outside the topic
   root. Add derived metadata only when supported by existing data; rebuild
   indexes from valid frontmatter and recompile only affected articles.
5. Validate the staged set together, then commit it atomically. Append the log
   entry as the final action. Any failure MUST roll back every migration change.
6. Prove the before/after raw history is intact, every source-backed article has
   resolvable provenance, freshness gaps remain visible, and normal query bounds
   still hold. A failed check leaves the entire topic unchanged.
