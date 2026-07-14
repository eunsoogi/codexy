# Wiki Directory Structure

> **Configurable hub path**: The hub location is read from `~/.config/llm-wiki/config.json` (`hub_path` field preferred; `resolved_path` is a legacy fallback). If no config exists, `~/wiki/` is the fallback. Throughout this document, `HUB/` means "the resolved hub path". See [hub-resolution.md](../hub-resolution.md) for the full resolution protocol (tilde expansion, space handling, iCloud paths).

## Hub (HUB/)

The hub is lightweight and MUST NOT contain content directories. It only tracks topic wikis.

```
HUB/                               # resolved from ~/.config/llm-wiki/config.json
├── wikis.json                     # Registry of all topic wikis
├── _index.md                      # Lists topic wikis with stats
├── log.md                         # Global activity log
└── topics/                        # Each topic is a full wiki
    ├── dementia/
    ├── quantum-computing/
    ├── .archive/                  # Archived topic wikis, hidden by default
    │   └── old-topic/
    └── ...
```

## Topic Sub-Wiki (HUB/topics/<name>/)

All content lives here. Init creates a core structure first; optional layers are
created lazily when a command needs them. This keeps new wikis fast to create
and avoids blank scaffolding for inventory, datasets, and generated sidecars
that may remain unused.

```
HUB/topics/<name>/
├── .obsidian/                     # Optional Obsidian vault config
├── _index.md                      # Master index: stats, quick nav, recent changes
├── .librarian/                    # Optional: wiki-only maintenance reports
│   ├── REPORT.md
│   └── scan-results.json
├── .audit/                        # Optional: umbrella audit reports
│   ├── REPORT.md
│   └── scan-results.json
├── config.md                      # Title, scope, conventions
├── log.md                         # Topic-level activity log
├── inbox/                         # Drop zone for this topic
│   └── .processed/
├── inventory/                     # Lazy: durable tracking records (see inventory.md)
│   ├── _index.md
│   ├── items/                     # Physical/digital items, parts, tools, assets
│   │   ├── _index.md
│   │   └── *.md
│   ├── candidates/                # Ingest candidates, tasks, questions, watch items
│   │   ├── _index.md
│   │   └── *.md
│   ├── entities/                  # People, orgs, projects, venues, standards bodies
│   │   ├── _index.md
│   │   └── *.md
│   ├── corpora/                   # Source collections, archives, datasets, forums
│   │   ├── _index.md
│   │   └── *.md
│   └── views/                     # Derived chat/list views over inventory
│       ├── _index.md
│       └── *.md
├── datasets/                      # Lazy: dataset manifests for large/external data
│   ├── _index.md
│   └── <dataset-slug>/
│       ├── _index.md
│       ├── MANIFEST.md
│       ├── samples/_index.md      # Lazy: created by dataset sample
│       ├── profiles/_index.md     # Lazy: created by dataset profile
│       └── queries/_index.md      # Lazy: created for query recipes
├── raw/                           # Immutable source material
│   ├── _index.md
│   ├── articles/
│   │   ├── _index.md
│   │   └── *.md
│   ├── papers/
│   │   ├── _index.md
│   │   └── *.md
│   ├── repos/
│   │   ├── _index.md
│   │   └── *.md
│   ├── notes/
│   │   ├── _index.md
│   │   └── *.md
│   └── data/
│       ├── _index.md
│       └── *.md
├── wiki/                          # Compiled articles (LLM-maintained)
│   ├── _index.md
│   ├── concepts/
│   │   ├── _index.md
│   │   └── *.md
│   ├── topics/
│   │   ├── _index.md
│   │   └── *.md
│   ├── references/
│   │   ├── _index.md
│   │   └── *.md
│   └── theses/                    # Thesis investigations
│       ├── _index.md
│       └── *.md
└── output/                        # Generated artifacts
    ├── _index.md
    ├── projects/                  # Project folders (see projects.md)
    │   ├── <slug>/
    │   │   ├── WHY.md             # Required: goal + rationale in plain markdown
    │   │   ├── *.md               # Markdown deliverables
    │   │   ├── *.png, *.svg       # Colocated images/diagrams
    │   │   ├── code/              # Optional — prototype scripts
    │   │   └── data/              # Optional — CSVs, JSON exports
    │   └── .archive/              # Archived projects (moved here by /wiki:project archive)
    │       └── <slug>/
    │           └── WHY.md
    └── *.md                       # Loose outputs (backward compatible)
```

See [inventory.md](../inventory.md) for inventory records, [datasets.md](../datasets.md)
for dataset manifests, and [projects.md](../projects.md) for the full projects
architecture (lifecycle, multi-membership, explicit `--project <slug>` scoping).
Files under `inventory/views/` are derived list/table views. They are not
inventory records and MUST NOT be treated as authoritative tracking state.
Missing optional roots (`inventory/`, `datasets/`, `.obsidian/`, `.librarian/`,
or `.audit/`) mean the layer has not been used yet.

## Local Wiki (--local flag)

Same structure as above but rooted at `<project>/.wiki/` without `wikis.json` or `topics/`.

## Wiki Resolution Order

When a command runs, first resolve the hub path (HUB) from `~/.config/llm-wiki/config.json` (see `hub-resolution.md`). Then resolve which wiki to use:

1. `--local` flag present → `<cwd>/.wiki/`
2. `--wiki <name>` flag present → look up name in `HUB/wikis.json`; MUST resolve `<HUB>`, leading `~`, absolute, and HUB-relative paths, and fall back to `HUB/topics/<name>` when a registry path is stale
3. Current directory has `.wiki/` → use it
4. Otherwise → HUB

## wikis.json Format

```json
{
  "default": "<HUB>",
  "wikis": {
    "hub": { "path": "<HUB>", "description": "Global knowledge base" },
    "<topic>": { "path": "topics/<topic>", "description": "...", "status": "active" },
    "<archived-topic>": {
      "path": "topics/.archive/<archived-topic>",
      "description": "...",
      "status": "archived",
      "archived": "YYYY-MM-DD",
      "archive_reason": "optional"
    }
  },
  "local_wikis": [
    { "path": "/absolute/path/.wiki", "description": "..." }
  ]
}
```

Topic paths inside the shared hub MUST be relative (`topics/<topic>`) or use
the `<HUB>` token. MUST NOT store `/Users/<name>/...` absolute paths for
hub-owned topic wikis; those break when an iCloud wiki is opened from another
Mac with a different home directory.

Archived topic wikis live under `topics/.archive/<slug>` and MUST keep their
registry entries with `status: archived`. Normal wiki resolution, status,
query, compile, research, output, librarian, refresh, and audit workflows skip
archived entries unless the user explicitly includes archived content. See
[archive.md](../archive.md) for lifecycle semantics and restore rules.

## _index.md Format

Every existing wiki-managed directory has an `_index.md`. This is the agent's
primary navigation aid. Placeholder indexes are unnecessary for optional
directories before they exist.

```markdown
# [Directory Name] Index

> [One-line description of what this directory contains]

Last updated: YYYY-MM-DD
