# Issue #558 — self-contained Sentinel packet

Packet version: `self-contained-review-v1`
Frozen review source head: `41b34e570a127acd26a3731d4948cf2ce94320a2`
Base: `0655cff6a084494905bee3b9b47f0cf50d8cba00`
Branch: `eunsoogi/558-component-status-doctor-bootstrap`
Issue owner: the existing child-owned #558 lane

Ownership metadata source: parent-supplied
Lane ownership: child-owned
Workflow profile: strict
Durable delegation: yes
Explicit audit evidence: requested

| Field | Value |
| --- | --- |
| Lane type | validation/QA |
| Secondary surfaces | Plugin package, wheel CLI, workflow, MCP/LSP exposure, selected review |
| Owner decision | affirmative child-owned because this is the established #558 worktree and branch |
| Atomic scope | One evidence-only packet commit. Production and test behavior are non-goals. |
| TDD boundary | No new engineering boundary. This packet binds the existing faithful RED/GREEN proof. |
| Review budget | Earlier full Sentinel result: `UNOBSERVABLE` because its packet was not inspectable. Parent authorization `EVIDENCE_PACKET_AND_NEW_FULL_REVIEW_AUTHORIZATION|558|41b34e57|self-contained-review-v1` permits exactly one new full Sentinel and no delta. |
| First review action | Inspect this packet only; the reviewer must not execute commands or mutate state. |

## Acceptance scope and non-goals

Issue #558 adds read-only `getcodexy status`, `doctor`, and bootstrap orchestration over
the installed Codex plugin inventory. Status and doctor must classify actual installed
plugins; doctor must issue only repairs accepted for that state. Bootstrap remains the
#557-owned transactional operation. #555 remains owner of identity, source, version,
and dependency admission.

This repair addresses the current security boundary only: every diagnostic descendant
must be admitted with no-follow semantics before status or doctor reads it. It does not
change lifecycle mutation, journal/replay, component resolution policy, public schema,
version, marketplace metadata, workflow behavior, or GitHub state.

## Exact frozen tree and base-to-head inventory

Raw readback:

```text
$ git status --short --branch
## eunsoogi/558-component-status-doctor-bootstrap...origin/main [ahead 21]
$ git rev-parse HEAD
41b34e570a127acd26a3731d4948cf2ce94320a2
$ git merge-base HEAD origin/main
0655cff6a084494905bee3b9b47f0cf50d8cba00
$ git diff --check origin/main...HEAD
(exit 0; no output)
$ git diff --stat origin/main...HEAD
25 files changed, 1756 insertions(+), 44 deletions(-)
```

Complete base-to-head file list (`git diff --name-status origin/main...HEAD`):

```text
M .github/workflows/python-package.yml
M docs/getcodexy-component-installation.md
A docs/review-control/issue-558-admission-before-repair-v1.json
A docs/review-control/issue-558-canonical-fallback-wheel-provenance-v1.json
A docs/review-control/issue-558-descendant-no-follow-v1.json
A docs/review-control/issue-558-executed-corruption-mcp-boundary-v1.json
M packages/getcodexy/src/codexy_runtime_tools/component_cli.py
A packages/getcodexy/src/codexy_runtime_tools/component_diagnostic_health.py
A packages/getcodexy/src/codexy_runtime_tools/component_diagnostic_surfaces.py
A packages/getcodexy/src/codexy_runtime_tools/component_inspection.py
M packages/getcodexy/src/codexy_runtime_tools/component_lifecycle.py
M packages/getcodexy/src/codexy_runtime_tools/component_lifecycle_preflight.py
M packages/getcodexy/src/codexy_runtime_tools/component_manifest.py
A packages/getcodexy/src/codexy_runtime_tools/component_observed_inventory.py
M packages/getcodexy/src/codexy_runtime_tools/component_resolver.py
A packages/getcodexy/src/codexy_runtime_tools/component_source_admission.py
M packages/getcodexy/src/codexy_runtime_tools/component_transition_model.py
M packages/getcodexy/src/codexy_runtime_tools/component_transition_rejections.py
A packages/getcodexy/tests/test_component_bootstrap.py
M packages/getcodexy/tests/test_component_cli.py
A packages/getcodexy/tests/test_component_diagnostic_surfaces.py
A packages/getcodexy/tests/test_component_inspection.py
A packages/getcodexy/tests/test_component_marketplace_fallback.py
M packages/getcodexy/tests/test_component_transition_model.py
M packages/getcodexy/tests/test_public_activation_contract.py
```

The descendant repair itself is commit `5e2a1098`; its proof ledger was committed as
`77c9b136`, and the Warden/Auditor receipts as `41b34e57`. The remaining base-to-head
commits are the earlier parent-authorized #558 status/doctor/bootstrap implementation
and repair history. The governed current source/test LOC is: source admission 185,
resolver 250, diagnostic surfaces 142, inspection 152, health 65, marketplace-fallback
test 243, diagnostic-surface test 75, inspection test 250. No governed file exceeds 250.

## Current source evidence

The following current bytes are the review-critical implementation. Their SHA-256 values
are: `component_source_admission.py` `a8db78e0aa0422b7663cd87a1b21754c3a69efd861f36b96d06031d42626a4cd`;
`component_resolver.py` `bbc6bd913528b0943f5df5ce79752c9b01edb0f1f24a15730a26555048176092`;
`component_diagnostic_surfaces.py` `94f1f78e03692fd50909d45759eae41c0bbe2b166debda7387dfb1110fc0d1fb`;
`component_inspection.py` `ff544d11effb349309329a645a5de8bd55416f505fdc8b15ada1a208b5f12cb5`;
`component_diagnostic_health.py` `d1f62fbb5cdf6eaad88912511f57a3aeadeade1ac56b403a588de103dabcfea5`.

```python
# component_source_admission.py — diagnostic path set and public handle
DIAGNOSTIC_PATHS = {
    "core": ("agents/catalog.toml", "agents/codexy-architect.toml",
             "agents/codexy-cartographer.toml", "agents/codexy-auditor.toml",
             "agents/codexy-shipwright.toml", "agents/codexy-inspector.toml",
             "agents/codexy-sentinel.toml", "agents/codexy-warden.toml",
             "hooks/hooks.json", "hooks/codexy-thread-delivery.sh",
             "hooks/codexy-thread-delivery.cmd"),
    "github": ("agents/catalog.toml", "agents/codexy-weaver.toml",
               "hooks/hooks.json", "hooks/codexy-github-workflow-context.sh",
               "hooks/codexy-github-workflow-context.cmd",
               "hooks/codexy-github-admission.sh",
               "hooks/codexy-github-admission-issue.cmd",
               "hooks/codexy-github-admission-pr.cmd"),
    "devtools": (".mcp.json", "mcp/codexy-mcp-devtools"),
}

@dataclass(frozen=True)
class DiagnosticTree:
    root: Path
    def read_regular(self, relative: str) -> bytes | None:
        try: return _read_regular(self.root, _relative(relative))
        except (OSError, ValueError): return None
    def executable(self, relative: str) -> bool:
        try: return bool(_metadata(self.root, _relative(relative)).st_mode & 0o111)
        except (OSError, ValueError): return False
    def present_or_unsafe(self, relative: str) -> bool:
        try: _path_metadata(self.root, _relative(relative)); return True
        except FileNotFoundError: return False
        except (OSError, ValueError): return True
    def admits(self, relatives: tuple[str, ...]) -> bool:
        try: return all(stat.S_ISREG(_path_metadata(self.root, _relative(path)).st_mode) for path in relatives)
        except (OSError, ValueError): return False

def diagnostic_paths(component: Component) -> tuple[str, ...]:
    return tuple(dict.fromkeys((*component.asset.required_paths, *DIAGNOSTIC_PATHS[component.id])))
```

```python
# component_source_admission.py — no-follow and containment implementation
def _read_regular(root: Path, relative: Path) -> bytes:
    target = _path_metadata(root, relative)
    if not stat.S_ISREG(target.st_mode): raise ValueError("diagnostic path is not a regular file")
    descriptor = _open_regular(root, relative)
    try:
        opened, unchanged = os.fstat(descriptor), _path_metadata(root, relative)
        same_file = (opened.st_dev, opened.st_ino) == (target.st_dev, target.st_ino)
        if not stat.S_ISREG(opened.st_mode) or not same_file or unchanged != target:
            raise OSError("diagnostic path changed while reading")
        return os.read(descriptor, opened.st_size)
    finally: os.close(descriptor)

def _path_metadata(root: Path, relative: Path) -> os.stat_result:
    _safe_directory(root)
    current = root
    for part in relative.parts[:-1]:
        current /= part; _safe_directory(current)
    metadata = os.lstat(current / relative.name)
    if _reparse(metadata): raise ValueError("diagnostic path is linked or reparse")
    return metadata

def _open_regular(root: Path, relative: Path) -> int:
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    if os.name == "nt": return os.open(root.joinpath(relative), flags | getattr(os, "O_BINARY", 0))
    directory_flags, descriptor = flags | getattr(os, "O_DIRECTORY", 0), os.open(root, flags | getattr(os, "O_DIRECTORY", 0))
    try:
        for part in relative.parts[:-1]:
            next_descriptor = os.open(part, directory_flags, dir_fd=descriptor)
            os.close(descriptor); descriptor = next_descriptor
        return os.open(relative.name, flags, dir_fd=descriptor)
    finally: os.close(descriptor)

def _relative(value: str) -> Path:
    relative = Path(value)
    if relative.is_absolute() or not relative.parts or any(part in {"", ".", ".."} for part in relative.parts):
        raise ValueError("diagnostic path is not relative")
    return relative

def _safe_directory(path: Path) -> None:
    metadata = os.lstat(path)
    if not stat.S_ISDIR(metadata.st_mode) or _reparse(metadata): raise ValueError("diagnostic path is not a real directory")

def _network_path(path: Path) -> bool:
    if str(path).replace("\\", "/").startswith("//"): return True
    if os.name != "nt" or not path.drive: return False
    return ctypes.windll.kernel32.GetDriveTypeW(f"{path.drive}\\") == 4
```

```python
# component_resolver.py — #555-owned inspection admission extension
def admit_inspected_inventory(manifest, inventory, marketplace_root):
    selected = admit_installed_inventory(manifest, inventory, marketplace_root)
    if marketplace_root is None: trees = {}
    else:
        components = tuple(manifest.component(component) for component in selected)
        if any(not trusted_component_root(marketplace_root, component) for component in components):
            raise ComponentResolutionError("conflicting-installed-state")
        trees = {component.id: DiagnosticTree(marketplace_root / component.asset.package_root) for component in components}
    if any(not trees[component].admits(diagnostic_paths(manifest.component(component))) for component in selected):
        raise ComponentResolutionError("conflicting-installed-state")
    return InspectedInstalledInventory(selected, trees)
```

```python
# component_inspection.py — #558 receives handles only
def _actual(manifest, installed, root):
    actual, records, _ = _observed(manifest, installed)
    try:
        admitted = admit_inspected_inventory(manifest, installed, root)
        if root is None: return admitted.selection, {}, {}, None
        if actual != admitted.selection: raise ComponentResolutionError("inconsistent-installed-state")
        return actual, records, admitted.trees, None
    except ComponentResolutionError as error: return actual, records, {}, error.code
    except (OSError, ValueError): return actual, records, {}, "invalid-installed-inventory"
```

```python
# health and registration code take DiagnosticTree, never a raw plugin path
def _stale(manifest, component, tree):
    if tree is None: return True
    if any(tree.read_regular(path) is None for path in manifest.component(component).asset.required_paths): return True
    return not _manifest_is_valid(tree, manifest.component(component).plugin, manifest.version) or not valid_surface(tree, component) or _legacy_core_monolith(tree, component)

def valid_surface(tree, component):
    if any(tree.read_regular(path) is None for path in SURFACE_PATHS[component]): return False
    if component == "devtools": return tree.executable("mcp/codexy-mcp-devtools") and _json_value(tree, ".mcp.json") == MCP
    if _toml_value(tree, "agents/catalog.toml") != CATALOGS[component]: return False
    if any(tree.read_regular(f"agents/{name}") is None for name in CATALOGS[component]["agent_files"]): return False
    return _json_value(tree, "hooks/hooks.json") == HOOKS[component]
```

## RED/GREEN mapping

| Finding ID | Faithful RED | Repair | Current GREEN and exact test evidence |
| --- | --- | --- | --- |
| `ee59a682-P1` intermediate descendant link | Historical `ee59a682` `valid_surface(plugin, "core")` returned `historical_valid_surface=True` after `agents/` was renamed and replaced by a symlink; the safety assertion failed (exit 1). SHA-256 `be184d7aaafe94aca83740330066ef523bb5e98f05292c7787048e7e12c051c6`. | `DiagnosticTree` admission and descriptor-relative traversal above. | `MarketplaceFallbackAdmissionTests.test_intermediate_diagnostic_links_are_rejected_before_health_reads`; covers core agents/hooks/.codex-plugin/assets, GitHub agents/hooks/.codex-plugin/skills/skills-git-workflow, devtools .codex-plugin/mcp. |
| `ee59a682-P1` terminal unsafe node | A terminal linked, directory, or special node could bypass root-only provenance. | `admit()` requires regular paths before diagnostic health; read also revalidates file identity. | `DiagnosticSurfaceTests.test_managed_registration_files_fail_closed_for_symlink_and_special_paths`; `MarketplaceFallbackAdmissionTests.test_terminal_special_node_is_rejected_before_health_reads` (POSIX FIFO). |
| `ee59a682-P1` Windows/provenance | Root-only check did not prove descendant reparse/remote failure. | Every segment checks reparse; UNC and remote drives reject. | `MarketplaceFallbackAdmissionTests.test_windows_reparse_and_remote_admission_fail_before_diagnostic_reads`; `...test_supported_windows_reparse_and_unc_provenance_are_rejected_without_path_api_support`; existing Windows integrity ancestor controls. |
| Earlier #558 admission-first findings | Invalid observations must not get stale/bootstrap advice. | Existing #555 `admit_installed_inventory` stays before tree admission and health. | `...test_successful_marketplace_admission_precedes_every_version_repair`; fallback canonical and invalid-state tests; health only emits bootstrap for admitted missing/stale state. |
| Earlier #558 behavioral registration findings | Presence-only MCP/hook/catalog checks are insufficient. | `valid_surface(DiagnosticTree, component)` reads canonical content plus launcher executable bit. | `DiagnosticSurfaceTests.test_canonical_managed_registrations_are_healthy`, `...test_devtools_mcp_requires_exact_lsp_and_codegraph_bindings`, `ComponentInspectionTests.test_doctor_requires_canonical_catalog_hooks_and_mcp_bindings`. |

Raw current GREEN terminal output:

```text
$ PYTHONPATH=packages/getcodexy/src:packages/getcodexy/tests python3 -m unittest discover -v -s packages/getcodexy/tests -p 'test_component_*.py'
Ran 127 tests in 1.226s
OK
SHA-256 complete raw log: c02fb957c68eecf1548fcecbcfaba2084bcb3db578adec404072df94cbec6fdc

$ PATH=<isolated python-to-python3 shim> PYTHONPATH=packages/getcodexy/src:packages/getcodexy/tests python3 -m unittest discover -s packages/getcodexy/tests
Ran 216 tests in 40.258s
OK
SHA-256 complete raw log: cf67c9107f1ea817deebf8b316c9a30122aa349bf807cc7d221127cccc90a449
The shim is only needed because this host lacks a `python` executable; it does not change package bytes.

$ cargo test --manifest-path packages/codexy-runtime/Cargo.toml getcodexy_component_contract --lib
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out
SHA-256 complete raw log: a9a0578930384a8cc88493ca131afdc62a856854e4238a24c5d77878d400076f
```

## Static and package proof

```text
$ scripts/validate-plugin-config --check
plugin config validation ok: plugins/codexy
repository GitHub policy validation ok

$ scripts/sync-plugin-version --check
plugin version sync ok: codexy=1.3.0, codexy-github=1.3.0, codexy-devtools=1.3.0

$ scripts/validate-plugin-config --check-touched-loc --base-ref origin/main
plugin config validation ok: plugins/codexy

$ ruby -e 'require "yaml"; YAML.load_file(".github/workflows/python-package.yml"); puts "workflow-yaml=ok"'
workflow-yaml=ok
```

Raw-output hashes in command order: plugin `61a27078ebe2c4abf5b4286d2a5804199e83f0d6cfd4457ce5f3e1d15057a9e9`,
version `e91ccd93e8a7d7ceee0302f3439b3cb3da74ea1445346fc897325256f5cbf8d8`,
LOC `3655634ab1eb52545817921d9a8b717783adef5d4cabe53cb840564973181a06`,
workflow parser `b2aec4d8fceec662682af17f33a4fec57cc0e918208178242dfad36c4a07069f`.

An isolated wheel was built from this tree, installed with `uv pip --no-index`, and
driven through its real `getcodexy` console entry point against a controlled Codex host.
All build, venv, install, six human/JSON command calls, and final status/doctor calls
exited `0`; assertion output was `wheel-console-assertions=PASS`
(`64be714feb59229cff79845f4b81b9d9605a7d3c47802268173a551302be1f1c`).

```json
{"command":"status","errors":[],"installed_components":[],"inventory":{"state":"absent"},"inventory_consistency":"not-recorded","outcome":"completed","schema":"getcodexy.status.v1","selected_components":[],"source_of_truth":"installed-component-inventory"}
{"command":"doctor","component_health":[],"errors":[],"host_readiness":{"missing_requirements":[],"state":"ready"},"inventory":{"state":"absent"},"inventory_consistency":"not-recorded","outcome":"completed","schema":"getcodexy.doctor.v1","source_of_truth":"installed-component-inventory"}
{"command":"bootstrap","errors":[],"installed_components":["core","github","devtools"],"outcome":"completed","requested_components":[],"resolved_components":["core","github","devtools"],"schema":"getcodexy.operation-receipt.v1","selection_after":["core","github","devtools"],"selection_before":[],"source_of_truth":"installed-component-inventory"}
{"command":"status","errors":[],"installed_components":["core","github","devtools"],"inventory":{"components":["core","github","devtools"],"state":"present"},"inventory_consistency":"consistent","outcome":"completed","schema":"getcodexy.status.v1","selected_components":["core","github","devtools"],"source_of_truth":"installed-component-inventory"}
{"command":"doctor","component_health":[{"component":"core","state":"healthy"},{"component":"github","state":"healthy"},{"component":"devtools","state":"healthy"}],"errors":[],"host_readiness":{"missing_requirements":[],"state":"ready"},"inventory":{"components":["core","github","devtools"],"state":"present"},"inventory_consistency":"consistent","outcome":"completed","schema":"getcodexy.doctor.v1","source_of_truth":"installed-component-inventory"}
```

The corresponding JSON-output SHA-256 values are status `63e73a842b21c2eb9defe1ba76c2aa93b358db274d300283c17b372cf9ed9a51`, doctor
`00eda0c1f09a5601d379573a7e2f585e658bb40fcb300a4732ffbff28a808574`, bootstrap
`2dd2b17d00ba2f9b0d0502b20e0630e934131ecacb7239cff9ef5c77b071044c`, final status
`3fe70837ff9e057bf2a1bbff17b2f44dca30b1ac2fd2ca995e310d60fc3d7123`, final doctor
`b344212214451827cf81263c8dd5105c8bb64e58552d63585db811dd57d545bb`.

## Independent review receipts

Warden: `warden_fallback_boundary`, current head `77c9b136`, assignment constrained it to
source-packet inspection with no command/test/network/GitHub/mutation/delegation. Terminal
verdict: **PASS**. Full verdict: “DiagnosticTree centralizes no-follow descendant admission
and reads: every component diagnostic path is admitted through lstat-based directory/reparse
checks, POSIX descriptor-relative O_NOFOLLOW traversal, Windows post-open metadata checks,
and before/after device-inode verification. UNC, remote-drive, root-escape, alias, symlink,
reparse, directory, FIFO, and special-file paths fail as conflicting-installed-state before
byte reads. component_inspection, diagnostic health, and diagnostic surfaces consume only
resolver-admitted trees; raw path access is removed. Existing #555 identity/dependency
admission remains authoritative, and #557 lifecycle mutation continues using unchanged
admit_operation_inventory.”

Auditor: `auditor_final_proof`, current head `77c9b136`, same no-command/no-mutation/no-
network/no-delegation constraint. Terminal verdict: **PASS**. Full verdict: “acceptance is
satisfied... Status/doctor can read diagnostic contents only through resolver-admitted
DiagnosticTree handles... Intermediate and terminal links/reparse points, unsafe directories,
FIFOs, UNC paths, and remote Windows drives fail closed as typed
conflicting-installed-state; health is incompatible/manual repair, never bootstrap...
Bootstrap remains the separate #557 transactional command... Exact gap: none within the
provided direct-inspection packet.”

## Tool exposure and review context

`codex mcp list --json` returned only `aside`, `computer-use`, `lazyweb`, `node_repl`, and
`openaiDeveloperDocs` (raw JSON SHA-256 `fb1575e1e49b84b1c147accdddecc6be8a405f298f9405ba040f1da92ada8453`).
Codexy codegraph/LSP were therefore unavailable; direct source inspection was used and the
exposure mismatch is carried here. No PR exists and no GitHub operation was authorized or
performed in this phase. The parent owns publication, current-head CI, connector review, and
merge routing after a Sentinel PASS.

## Goal and plan receipts

Raw goal-tool receipt before this phase: `{"goal":null,"remainingTokens":null,
"completionBudgetReport":null}`. Exact `create_goal` result:
`{"goal":{"objective":"Build and verify a self-contained evidence packet for issue #558 on
clean head 41b34e57, commit only that packet, and obtain the one parent-authorized new full
Sentinel review.","status":"active","tokensUsed":0,"timeUsedSeconds":0}}`. The finite goal
is active while this packet is prepared. The `update_plan` result was `{}` after accepting these
ordered steps: collect/verify evidence; create packet; commit packet/refresh static proof;
Sentinel; handoff. Before Sentinel wait the lane will complete this finite goal and wait idle.

The prior terminal finding is fully consumed by this packet. No runtime or test behavior
changes are authorized in this phase. The next parent-owned action after a Sentinel PASS is
publication/current-head-CI authorization; BLOCK or UNOBSERVABLE is terminal and receives no
automatic reviewer retry.
