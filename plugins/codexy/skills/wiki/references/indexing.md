# Indexing Protocol

## Purpose

Index files (`_index.md`) are the agent's navigation system. Instead of scanning hundreds of files, the agent reads a single index to find what it needs. This is the key efficiency mechanism.

## The 3-Hop Strategy

When answering a query or finding content:

1. **Hop 1**: MUST read master `_index.md` → get overview, identify which section is relevant
2. **Hop 2**: MUST read `wiki/{category}/_index.md` → scan summaries and tags for matches
3. **Hop 3**: MUST read only the matched article files

This means the agent typically reads 2-3 small index files + 3-8 full articles, rather than scanning dozens of files.

## Derived Index Protocol

**Indexes are a cache, not a source of truth.** The `.md` files and their YAML frontmatter are the source of truth. Indexes are rebuilt on read when stale. This makes the wiki concurrent-safe — multiple sessions can write simultaneously without locks.

### Stale Detection

Before using any `_index.md`, MUST check staleness:

1. Count `.md` files in the directory (excluding `_index.md`)
2. Count rows in the `_index.md` contents table
3. If counts differ → index is stale → rebuild inline before proceeding

If an optional layer such as `inventory/` or `datasets/` is completely absent,
MUST treat its count as 0 instead of creating a placeholder index during a read.

### Rebuild Inline

When an index is stale:

1. MUST list all `.md` files in the directory (excluding `_index.md`)
2. MUST read each file's YAML frontmatter (title, summary, tags, updated)
3. MUST regenerate the `_index.md` contents table from frontmatter
4. MUST recalculate statistics (source count, article count, etc.)
5. MUST write the new `_index.md`
6. MUST continue with the original operation

### Write Operations (ingest, compile, research, inventory, dataset)

- MUST write the article/source file with correct frontmatter — this is the source of truth
- Index updates are **best-effort** — update if convenient, but if skipped or if a concurrent session overwrites, no data is lost
- The next read will detect staleness and rebuild

### Read Operations (query, status, lint)

- MUST always stale-check before trusting the index
- If stale, rebuild first, then proceed
- This adds a small overhead on first read after writes, but guarantees accuracy

### Why This Works for Concurrency

- Two sessions writing articles simultaneously: both write files, neither corrupts the other
- Index may be momentarily stale or one rebuild may overwrite another's rebuild — but since both rebuild from the same source files on disk, the result converges to the same correct state
- `log.md` is append-only with small atomic writes — already safe
- No locks needed, no stale lock cleanup, no coordination between sessions

## When to Update Indexes (Best-Effort)

Write operations SHOULD update indexes when convenient:
- A file is added to the directory
- A file is removed from the directory
- A file's frontmatter (title, summary, tags) changes
- Statistics change (after compilation, after lint)

But these updates are optional. If skipped (e.g., due to a crash or concurrent write), the next read operation will detect the stale index and rebuild it automatically.

## Index Update Procedure

### Adding a file

1. MUST read the current `_index.md`
2. MUST add a new row to the Contents table: `| [filename.md](filename.md) | Summary | tags | YYYY-MM-DD |`
3. If the file's tags introduce a new category, add it to the Categories section
4. MUST add entry to Recent Changes: `- YYYY-MM-DD: Added filename.md (brief note)`
5. MUST update "Last updated" date

### Removing a file

1. MUST read the current `_index.md`
2. MUST remove the row from Contents table
3. MUST remove from Categories if it was the only file with that category
4. MUST add removal entry to Recent Changes
5. MUST update "Last updated" date

### Master Index Statistics

The root `_index.md` statistics are derived from actual file counts, not manual tracking:
- Sources: count .md files in `raw/` subdirectories (excluding `_index.md`)
- Articles: count .md files in `wiki/` subdirectories (excluding `_index.md`)
- Inventory records: count .md files in `inventory/` subdirectories (excluding `_index.md`), or 0 if `inventory/` is absent
- Dataset manifests: count `datasets/*/MANIFEST.md`, or 0 if `datasets/` is absent
- Outputs: count .md files in `output/` (excluding `_index.md`)
- Hub archived topics: count `HUB/topics/.archive/*/_index.md` or registry
  entries with `status: archived`. Archived topics are reported separately and
  excluded from active topic counts/tables by default.

## Cross-Wiki Index Peek

When peeking at sibling wikis for overlap:
1. MUST read `HUB/wikis.json` to get the list of all wikis
2. MUST skip entries with `status: archived` or paths under `topics/.archive/` unless
   the query is deep enough to report archived matches or the user passed
   `--include-archived`
3. For each active sibling wiki, MUST read ONLY its `_index.md` (not full articles)
4. MUST check if any summaries or tags match the current query
5. If overlap is found, MUST report it in the response and MUST limit sibling
   wiki reads to `_index.md` files unless explicitly asked for full articles.
6. For deep queries, MUST report archived sibling `_index.md` matches under an
   Archived Matches section. MUST NOT cite archived content as active
   evidence unless the user explicitly includes archived content.
