#!/bin/sh
set -eu

runtime_archive=${1:?activated runtime archive required}
bundle_archive=${2:?bundle archive required}
: "${RELEASE_TAG:?release tag required}"
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(dirname "$script_dir")
component_manifest="$root/packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
marketplace="$root/.agents/plugins/marketplace.json"
test -f "$runtime_archive"
test -f "$component_manifest"
test -f "$marketplace"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM
mkdir "$tmp_dir/extracted"
tar --no-same-owner --no-same-permissions -xzf "$runtime_archive" -C "$tmp_dir/extracted"

ROOT="$root" EXTRACTED="$tmp_dir/extracted" OUTPUT="$bundle_archive" RELEASE_TAG="$RELEASE_TAG" \
COMPONENT_MANIFEST="$component_manifest" MARKETPLACE="$marketplace" python3 - <<'PY'
import gzip, json, os, shutil, tarfile
from pathlib import Path

root = Path(os.environ["ROOT"])
staged = Path(os.environ["EXTRACTED"])
output = Path(os.environ["OUTPUT"])
target = os.environ["RELEASE_TAG"].removeprefix("v")
components = json.loads(Path(os.environ["COMPONENT_MANIFEST"]).read_text())["components"]
marketplace = json.loads(Path(os.environ["MARKETPLACE"]).read_text())
activation = json.loads((root / ".agents/plugins/runtime-activation.json").read_text())
runtime_platforms = list(activation["candidate"]["platforms"])
expected = [(item["id"], item["plugin"], item["asset"]["packageRoot"]) for item in components]
if expected != [("core", "codexy", "plugins/codexy"), ("github", "codexy-github", "plugins/codexy-github"), ("devtools", "codexy-devtools", "plugins/codexy-devtools")]:
    raise SystemExit("unsupported release-train component inventory")
if any(item["version"] != target for item in components):
    raise SystemExit("component manifest version does not match release tag")
if [(item["name"], item["source"]["path"].removeprefix("./")) for item in marketplace["plugins"]] != [(plugin, path) for _, plugin, path in expected]:
    raise SystemExit("marketplace inventory does not match component manifest")
if any(item["version"] != target for item in marketplace["plugins"]):
    raise SystemExit("marketplace version does not match release tag")
bundle = staged / "bundle"
for _, plugin, package_root in expected:
    source = staged / package_root if plugin == "codexy-devtools" else root / package_root
    destination = bundle / package_root
    if not source.is_dir():
        raise SystemExit(f"missing component source: {package_root}")
    shutil.copytree(source, destination, symlinks=True)
    if plugin == "codexy-devtools" and any((destination / name).exists() for name in ("runtime-candidate.json", "runtime-release.json")):
        raise SystemExit("release train may not retain runtime contracts")
    manifest = json.loads((destination / ".codex-plugin/plugin.json").read_text())
    if manifest.get("name") != plugin or manifest.get("version") != target:
        raise SystemExit(f"invalid component manifest: {package_root}")
    if plugin == "codexy-devtools" and manifest.get("supportedPlatforms") != runtime_platforms:
        raise SystemExit("public devtools manifest does not match activated runtime platforms")
    for required in next(item for item in components if item["plugin"] == plugin)["asset"]["requiredPaths"]:
        if not (destination / required).is_file():
            raise SystemExit(f"missing required component path: {package_root}/{required}")
marketplace_path = bundle / ".agents/plugins/marketplace.json"
marketplace_path.parent.mkdir(parents=True)
shutil.copy2(os.environ["MARKETPLACE"], marketplace_path)
for path in bundle.rglob("*"):
    if path.is_symlink():
        raise SystemExit(f"release train does not permit symlinks: {path.relative_to(bundle)}")
    if not path.is_file() and not path.is_dir():
        raise SystemExit(f"unsupported release train entry: {path.relative_to(bundle)}")
paths = [bundle / ".agents", bundle / ".agents/plugins", marketplace_path]
for _, _, package_root in expected:
    plugin = bundle / package_root
    paths.extend([plugin, *sorted(plugin.rglob("*"))])
output.parent.mkdir(parents=True, exist_ok=True)
with output.open("wb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed, tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as archive:
    for path in paths:
        relative = path.relative_to(bundle)
        info = archive.gettarinfo(str(path), str(relative))
        info.uid = info.gid = 0
        info.uname = info.gname = ""
        info.mtime = 0
        if relative.as_posix() == "plugins/codexy-devtools/mcp/codexy-mcp-devtools":
            info.mode = 0o755
        if info.isfile():
            with path.open("rb") as source:
                archive.addfile(info, source)
        else:
            archive.addfile(info)
PY
