# Repository linting

Run the read-only aggregate check from the repository root:

```sh
scripts/lint-repository --check
```

Apply only safe formatters and auto-fixers, then check again:

```sh
scripts/lint-repository --fix
scripts/lint-repository --check
```

On Windows, invoke the Python entry point:

```powershell
python scripts/lint-repository.py --check
```

The source-derived inventory covers Rust, Python, shell, PowerShell, Windows
command launchers, and Markdown, JSON, YAML, and TOML. Prettier checks only its
supported text formats; Taplo checks TOML. CI installs the pinned tools in
`tooling/lint-tools.json` and checks changed tracked source against the pull
request base, avoiding a repository-wide rewrite of existing lint debt.

Check mode is read-only. Fix mode applies Rustfmt, Ruff, shfmt,
`Invoke-Formatter`, Prettier, and Taplo. Windows command launchers are
check-only: their minimal non-executing contract avoids running checkout code.
Generated, vendor, and intentionally malformed fixture roots are excluded.

Run either command only in a trusted checkout: check mode can execute Cargo
build scripts and procedural macros, while fix mode can also change source. The
runner passes only Git-tracked regular-file paths to formatters.
