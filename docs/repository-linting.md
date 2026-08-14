# Repository linting

Run every read-only language check from the repository root:

```sh
scripts/lint-repository --check
```

Apply only safe formatters and auto-fixers, then check again:

```sh
scripts/lint-repository --fix
scripts/lint-repository --check
```

On Windows, run the same command through PowerShell:

```powershell
scripts/lint-repository.ps1 --check
```

`tooling/lint-tools.json` is the single tool-version policy. The runner
inventories Rust, Python, shell, PowerShell, Windows command launchers, and
Markdown, JSON, YAML, and TOML. Prettier checks Markdown, JSON, and YAML;
Taplo checks TOML. Rust uses the repository
toolchain and lockfile; the other CI tools use the exact versions listed there.

Check mode is read-only. Fix mode applies Rustfmt, Ruff, shfmt,
`Invoke-Formatter`, and Prettier; Ruff check mode is non-mutating while Ruff
fix mode includes `ruff check --fix` before formatting. Windows command launchers remain check-only
because running them to parse syntax could execute hooks and no safe canonical
formatter exists. `.prettierignore` and the runner exclude the exact generated,
vendor, and intentionally malformed fixture roots listed in the policy; source
and executable fixtures are still classified by suffix or shebang. JSONL is an
intentional check-only fixture/evidence format and is excluded from formatting.

Run either command only in a trusted checkout: check mode can execute Cargo
build scripts and procedural macros, while fix mode can also change source. The
runner intentionally refuses symlinks and
uses only NUL-safe Git-tracked regular-file paths, so it cannot follow an
untracked or out-of-repository path. Stage intended new files before running it.
