# Lint and format maintenance

This repository checks Rust, Python, POSIX shell, PowerShell, JSON, YAML, and
Markdown through `.github/workflows/language-lint.yml`. Keep local commands and
that workflow aligned; do not introduce a wrapper or alternate runner.

## Authoritative tools and files

- `tooling/lint-tools.json` records the Rust 1.95.0, dprint, ShellCheck, shfmt,
  and PSScriptAnalyzer pins. The workflow reads the Rust and dprint pins
  directly.
- `tooling/lint-requirements.txt` pins and hashes Ruff. Install it with
  `python -m pip install --disable-pip-version-check --only-binary=:all: --require-hashes -r tooling/lint-requirements.txt`.
- `dprint.json` configures JSON, YAML, and Markdown formatting.
- `packages/codexy-runtime/Cargo.toml` owns Rust lint severities. The production
  Clippy boundary is Rust 1.95.0 with `--locked --lib --bins`.
- `packages/getcodexy/pyproject.toml` and its tracked `uv.lock` define the
  Python package environment. Consume the lock with `uv lock --check` and
  `uv sync --locked`; CI MUST NOT generate it.

## Local commands

Use the selected Rust pin for Rust checks:

```sh
rust="$(jq -r .rust tooling/lint-tools.json)"
cargo +"$rust" fmt --manifest-path packages/codexy-runtime/Cargo.toml --all -- --check
cargo +"$rust" fmt --manifest-path packages/codexy-runtime/Cargo.toml --all
cargo +"$rust" clippy --manifest-path packages/codexy-runtime/Cargo.toml --locked --lib --bins
```

Validate and consume the tracked uv lock:

```sh
uv lock --check --directory packages/getcodexy
uv sync --locked --directory packages/getcodexy
```

Run Ruff after installing its pinned requirements. Directory discovery covers
`.py` files; Git-tracked Python shebang executables are explicit Ruff inputs so
extensionless legacy files cannot bypass checks:

```sh
ruff check .
git grep -lz '^#!.*python' | xargs -0r ruff check
ruff format --check .
git grep -lz '^#!.*python' | xargs -0r ruff format --check
ruff format .
git grep -lz '^#!.*python' | xargs -0r ruff format
```

Run the remaining language tools directly:

```sh
git ls-files -z '*.sh' | xargs -0r shellcheck
git ls-files -z '*.sh' | xargs -0r shfmt -d
git ls-files -z '*.sh' | xargs -0r shfmt -w
dprint check
dprint fmt
```

For PowerShell, install the pinned analyzer and run it directly over each
discovered `.ps1` path, matching CI:

```powershell
Install-Module PSScriptAnalyzer -RequiredVersion 1.25.0 -Scope CurrentUser -Force
$files = @(Get-ChildItem -Recurse -Filter *.ps1 -File | Select-Object -ExpandProperty FullName)
foreach ($file in $files) { Invoke-ScriptAnalyzer -Path $file -Severity ParseError,Error,Warning -EnableExit }
```

## CI, filenames, and scope

The `Language lint` workflow has one job each for Rust, Python, Shell, Text, and
PowerShell. They prove the direct commands above, including Rust formatting and
production Clippy, the tracked uv lock, Ruff coverage for Python shebang
executables, shell lint/format, dprint formatting, and PowerShell parse/error/
warning checks.

Maintained Python executables use lowercase `snake_case` names with `.py`;
maintained POSIX shell executables use lowercase `kebab-case` names with `.sh`.
Update every tracked caller when renaming one. Do not retain an extensionless
compatibility wrapper.

Formatter-touched governed files must stay at or below 250 lines. When a file
exceeds the limit, split cohesive responsibilities rather than adding a policy
exception or unrelated refactor.

Node, npm, Prettier, custom runners, parsers, and frameworks are excluded from
this lint and format surface.
