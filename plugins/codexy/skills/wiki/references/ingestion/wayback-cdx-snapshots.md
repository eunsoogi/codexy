### Wayback CDX Snapshots

MUST use this adapter for bounded archived web captures. The CDX API is the
inventory; MUST NOT recursively crawl live or archived HTML beyond the CDX result
set.

1. Accept either a CDX API URL or an original URL/prefix. For original URLs,
   query `https://web.archive.org/cdx` with JSON output, `fl=timestamp,original,statuscode,mimetype,digest,length`, and a conservative `filter=statuscode:200`.
2. MUST use `collapse=digest` by default to deduplicate captures with identical
   content. Respect `--from`, `--to`, `--include`, `--exclude`, and `--limit`
   if provided.
3. MUST fetch each selected capture with the `id_` replay form so the archived HTML
   is returned with minimal Wayback UI rewriting:
   `https://web.archive.org/web/<timestamp>id_/<original-url>`.
4. Convert HTML to markdown with a readability pipeline. Prefer a temporary
   Python virtual environment using `readability-lxml` plus `markdownify` or
   `html2text`; if dependency installation is unavailable, MUST use WebFetch with an
   extraction prompt and record the fallback in `extraction_tool`.
5. MUST write the manifest to `raw/repos/` and each readable snapshot to
   `raw/articles/`. MUST preserve snapshot provenance in frontmatter:
   `wayback_timestamp`, `wayback_original`, `wayback_digest`, `statuscode`,
   `mimetype`, `length`, `canonical_url`, and `extraction_tool`.
6. MUST skip captures whose body is empty, binary, or mostly navigation after
   readability extraction. Count and report skips by reason.

For volatile pages, later compilation MUST treat Wayback captures as evidence
of what an archived page said at a specific timestamp, not as evidence that the
claim remains true.

### Collection Compilation

After collection ingestion, compile selectively:

- Prefer synthesized clusters over one compiled article per page.
- MUST use reference articles for indexes/timelines/glossaries.
- For BIPs, likely clusters are activation mechanisms, wallet standards, script
  upgrades, peer services, Taproot/Schnorr, mining/RPC, and the BIP process.
- For community wikis, default confidence to `medium` unless corroborated by
  authoritative specs, code, papers, or multiple independent sources.

## URL Ingestion

1. **MUST Detect X.com / Twitter URLs**: If the URL matches `x.com/*/status/*` or `twitter.com/*/status/*`, MUST follow this fallback chain in order:

   **a) Grok MCP (preferred)**: MUST check if the `grok` MCP server is available by looking for tools matching `mcp__grok__*` (e.g., `mcp__grok__search`). If available, MUST use it to fetch the tweet/thread content. MUST extract: author handle, display name, full text, date, media descriptions, thread context.
   > Install: [github.com/nvk/ask-grok-mcp](https://github.com/nvk/ask-grok-mcp)

   **b) FxTwitter proxy**: If Grok MCP is not available, rewrite the URL:
   - `x.com/user/status/123` → `https://api.fxtwitter.com/user/status/123`
   - MUST WebFetch this API URL; it returns JSON with full tweet text, author, media, and thread data.
   - MUST parse the JSON response for `tweet.text`, `tweet.author`, `tweet.created_at`.

   **c) VxTwitter proxy**: If FxTwitter fails, try:
   - `x.com/user/status/123` → `https://api.vxtwitter.com/user/status/123`
   - Same JSON extraction as FxTwitter.

   **d) Direct WebFetch**: Last resort — WebFetch the original `x.com` URL. This often returns limited content (login walls), but sometimes works for public tweets.

   **e) Manual fallback**: If all above fail, report: "Could not fetch tweet content. Options: install [ask-grok-mcp](https://github.com/nvk/ask-grok-mcp) for X.com access, or paste the tweet text manually via `/wiki:ingest \"text\" --title \"@author tweet\"`."

   Type: notes (unless overridden).

2. **Detect PDF URLs**: If the URL ends in `.pdf` or returns a PDF content type,
   MUST download it to a temporary file and follow the PDF file ingestion flow.
   Type: papers by default unless the content is clearly legal/regulatory
   correspondence better treated as articles.

3. **GitHub repo URLs**: MUST use WebFetch with prompt:

   > "Extract from this GitHub repository: name, description, key technologies, main purpose, README content. Format as markdown."

4. **General URLs**: MUST use WebFetch to retrieve content. Prompt:

   > "Extract the complete article content from this page. Return: title, author(s) if listed, date published if listed, and the full article text preserving all factual claims, data points, code examples, and technical details. Format as clean markdown."

5. **Failure handling**: If WebFetch fails (auth wall, paywall), report the failure. MUST suggest: paste content manually via `/wiki:ingest "text" --title "Title"`.

## File Ingestion

1. MUST read the file directly
2. Markdown → preserve formatting
3. Plain text → wrap in markdown
4. PDF → extract to markdown using the PDF ingestion flow below
5. JSON/CSV/structured data → describe schema + representative sample for
   single-source ingest, or hand off to `ingest-collection --adapter
   csv-messages` when the user wants one source per row/message
6. Images → create a metadata stub noting the image path and any visible content description

### PDF Ingestion

PDFs are single-source ingests, not collection imports. MUST use them for court
filings, regulatory papers, academic PDFs, reports, and scanned documents whose
content MUST become one raw markdown source.

1. Determine source type:
   - `papers` for academic, technical, regulatory, or report-like PDFs.
   - `articles` for legal filings, court exhibits, notices, or web-published
     documents that are not papers.
2. Try `pdftotext -layout <pdf> -` only if it is available and produces
   non-trivial text. If local poppler is broken, missing, or returns garbled
   output, MUST NOT keep retrying it.
3. Fallback to a temporary Python virtual environment and a PDF library:
   - Prefer `pypdf` for text-first PDFs because it is lightweight.
   - MUST use `pymupdf` when layout fidelity or extraction quality matters.
   - MUST record `extraction_tool` and any dependency/version in frontmatter or a
     short provenance note.
4. If the PDF is image-only and no OCR tool is available, MUST create a metadata stub
   with `extraction_status: ocr-needed`, page count if detectable, file hash,
   and the original path/URL. MUST NOT invent text from the filename.
5. MUST preserve page boundaries in the body with `## Page N` headings when the
   extractor exposes them. MUST keep footnotes, docket numbers, tables, citations,
   and regulatory/legal identifiers intact.
6. MUST include extra frontmatter when known: `content_format: pdf`, `sha256`,
   `page_count`, `extraction_tool`, `extraction_status`, and `fetched` for URL
   PDFs.

## Freeform Text Ingestion

1. User provides quoted text as the argument
2. If `--title` not provided, derive a title from the first sentence or ask
3. Auto-tag based on content keywords

## Inbox Processing

The `inbox/` directory is a drop zone. Users dump files there via Finder, `cp`, etc.

### Processing `--inbox`:

1. Scan `inbox/` for all files (exclude `.processed/` subdirectory and hidden files)
2. For each file:
   - `.url` or `.webloc` files → extract the URL, then MUST follow URL ingestion flow
   - `.md` or `.txt` files → ingest as notes or articles (auto-detect)
   - `.pdf` files → extract to markdown with the PDF ingestion flow
   - `.json`, `.csv`, `.tsv` → ingest as data, or hand off to
     `ingest-collection --adapter csv-messages` for per-message sources
   - Other files → create a metadata stub noting file type and path
3. MUST move each processed file to `inbox/.processed/` (or delete if user did not pass `--keep`)
4. MUST report each item processed
5. If 5+ items were processed, MUST suggest: "You've ingested N new sources. Want me to compile? Run `/wiki:compile`"

## Slug Generation

1. Take the title, lowercase, replace spaces with hyphens, remove special characters
2. Prepend today's date: `YYYY-MM-DD-`
3. Truncate to 60 characters max (not counting .md extension)
4. Example: "Attention Is All You Need" → `2026-04-04-attention-is-all-you-need.md`
5. If a file with that slug already exists, MUST append `-2`, `-3`, etc.
6. This canonicalization applies to new ingests. If a legacy or imported raw
   file already exists with spaces, title case, or upstream naming, MUST NOT
   rename it during later maintenance; provenance workflows resolve exact paths
   and slug fallbacks per `wiki-structure.md` Source Reference Resolution.

## Post-Ingestion Index Updates

After writing each source file, MUST update indexes in order:

1. `raw/{type}/_index.md` — MUST add row to Contents table
2. `raw/_index.md` — MUST add row to Contents table
3. `_index.md` (master) — MUST increment source count, MUST add to Recent Changes

## Batch Ingestion

If the user provides multiple URLs or paths (comma-separated, space-separated, or one per line), process each sequentially. MUST report progress after each item.

## Compilation Nudge

After ingestion, count uncompiled sources (sources ingested after last compile date). If 5+, suggest running `/wiki:compile`.
