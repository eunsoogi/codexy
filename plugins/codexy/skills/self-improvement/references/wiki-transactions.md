# Wiki revisions and scoped promotion

The revision helper provides file integrity and serialized publication. It does
not judge evidence quality, privacy, destination authority, or rule adoption.
Before calling it, MUST validate those requirements under
[rule learning](rule-learning.md) and the destination owner's contract. MUST
keep project-only evidence out of global storage and MUST NOT interpret
successful storage as adoption or permission to change controlling instructions.

## Additive initialization and rollback

For one explicitly authorized existing Wiki root, first validate its provenance,
freshness and destination under [migration](wiki-migration.md). Then run
`python3 scripts/wiki_store_cli.py initialize <absolute-root>`. The helper
creates an additive `.codexy/` revision area, importing the root's `raw/`,
`wiki/`, `_index.md`, `log.md`, and `config.md` without changing the original
files. It refuses an existing revision area and linked or unsupported files.
MUST NOT delete or overwrite original data to retry. If initialization fails,
preserve the existing tree and report the gap.

Once initialized, `.codexy/CURRENT` identifies the authoritative revision for
this contract. Earlier snapshots and the original tree remain present. Returning
to a prior installation can still read the original tree; later changes remain
in their revision snapshots and MUST NOT be claimed to have been merged back
automatically. A rollback MUST retain these snapshots and report any changes
that need an authorized additive migration before they are used by older tools.

## Read and commit

For ordinary use, run
`python3 scripts/wiki_store_cli.py read <absolute-root>
--path <root-relative-path>`
to read one verified file at the current revision; the default per-file limit is
4,000 bytes, configurable with `--limit` only within the
[bounded query contract](wiki-minimal-contract.md). Search the index first, then
retrieve only relevant sources. The `verify` operation checks the entire
revision and is for explicit integrity verification, not every lookup. An
unavailable, changed, or oversized source MUST remain a visible gap.

To prepare a reviewable candidate JSON, provide a unique `id`, nonempty
`sources` array of source identities and revisions, `changes` mapping, one-line
`operation`, and an evidence `checkpoint`. Changes MUST include `_index.md` and
at least one `wiki/*.md` article or the local continuation checkpoint. New
`raw/*.md` revisions may be added; an existing raw revision MUST NOT be
replaced. MUST retain counterevidence in a new raw revision and preserve earlier
claims in prior snapshots. Changes to AGENTS, policies, settings and goals are
outside this interface.

For an issue-local continuation checkpoint, `changes` may include
`wiki/continuation.json` and an optional integer `brief_budget_bytes` at the
candidate's top level. The store validates the new brief and compares it to the
previous committed brief before publishing. Failed validation leaves the prior
revision current. MUST still reconcile cited corrections, resolutions, owner and
authority against live sources; a structural pass is not that proof.

After scoped validation, run
`python3 scripts/wiki_store_cli.py commit
<absolute-root> --expected <last-verified-revision> --candidate <candidate>`.
The helper serializes writers, checks the expected revision, commits a new
snapshot and its checkpoint together, and atomically replaces the current
pointer. Same-ID same-content retries are idempotent; a stale revision or
same-ID conflicting input fails without discarding the candidate. Re-read
relevant evidence and revalidate before retrying a conflict. An interrupted
write leaves the previous current pointer or the completed new pointer; an
unreferenced staged revision is not an adopted lesson and MUST NOT be deleted
without separate authority. The helper makes no guarantee against uncooperative
writers that edit its files outside its lock; such changes MUST fail integrity
checks and be reported.
