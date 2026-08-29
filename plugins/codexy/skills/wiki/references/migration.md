# Topic Migration

Migrate only one explicit existing topic root. Migration is additive and MUST
NOT delete, overwrite, or rename existing topic data.

1. Read the complete topic tree and snapshot its bytes.
2. Before any write, validate provenance and freshness using
   [Minimal Contract](minimal-contract.md). Any contract failure MUST stop
   migration, report the gap, and leave all topic bytes unchanged.
3. Apply the minimal contract's raw-history rules to every source change.
4. Stage derived changes and the contract-required operation record outside the
   topic root. Add derived metadata only when supported by existing data;
   rebuild indexes from valid frontmatter and recompile only affected articles.
5. Validate the staged set together, then commit it atomically, applying the
   operation record as the final action. Any failure MUST roll back every
   migration change.
6. Prove the minimal contract's raw-history, provenance, freshness, and query
   guarantees remain satisfied. A failed check leaves the entire topic
   unchanged.
