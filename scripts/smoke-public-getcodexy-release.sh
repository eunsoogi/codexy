#!/usr/bin/env bash
set -euo pipefail

: "${TARGET_VERSION:?}"
: "${RUNNER_TEMP:?}"

python -m venv public-bootstrap
if [[ -n "${GETCODEXY_DIST:-}" ]]; then
	public-bootstrap/bin/python -m pip install --no-cache-dir --no-index \
		--find-links "$GETCODEXY_DIST" "getcodexy==${TARGET_VERSION}"
else
	public-bootstrap/bin/python -m pip install --no-cache-dir \
		--index-url https://pypi.org/simple "getcodexy==${TARGET_VERSION}"
fi
public_inspect_root=${PUBLIC_INSPECT_ROOT:-public-inspect}
public_bundle_archive=${PUBLIC_BUNDLE_ARCHIVE:-public-bundle.tar.gz}
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/plugins/codexy-devtools" -- --help
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/$public_inspect_root/plugins/codexy-devtools" -- --help
mkdir -p public-marketplace
tar --no-same-owner --no-same-permissions -xzf "$public_bundle_archive" -C public-marketplace
git -C public-marketplace init -q
git -C public-marketplace config user.name "Codexy public proof"
git -C public-marketplace config user.email "codexy-public-proof@example.invalid"
git -C public-marketplace add --all
git -C public-marketplace commit -qm "public release proof"
git -C public-marketplace tag "v${TARGET_VERSION}"
cp scripts/fake_public_codex_host.py "$RUNNER_TEMP/codex"
chmod 755 "$RUNNER_TEMP/codex"
public_code_home="$RUNNER_TEMP/empty-codex-home"
mkdir "$public_code_home"
test -z "$(find "$public_code_home" -mindepth 1 -print -quit)"
proof_env=(env PATH="$RUNNER_TEMP:$PATH" CODEX_HOME="$public_code_home" CODEXY_RUNTIME_PLATFORM=linux-x86_64 CODEXY_MARKETPLACE_ROOT="$PWD/public-marketplace")
"${proof_env[@]}" public-bootstrap/bin/getcodexy install --json >public-install.json
jq -e '.schema == "getcodexy.operation-receipt.v1" and .outcome == "completed" and .errors == [] and (.selection_after | sort == ["core", "devtools", "github"])' public-install.json >/dev/null
"${proof_env[@]}" codex plugin list --json >public-plugin-inventory.json
jq -e --arg version "$TARGET_VERSION" '(.installed | length == 3) and ([.installed[] | select(.installed == true and .enabled == true and .version == $version)] | length == 3) and ([.installed[].name] | sort == ["codexy", "codexy-devtools", "codexy-github"])' public-plugin-inventory.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy status --json >public-status.json
jq -e '.schema == "getcodexy.status.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .errors == [] and ([.installed_components[]] | sort == ["core", "devtools", "github"])' public-status.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy doctor --json >public-doctor.json
jq -e --arg version "$TARGET_VERSION" '.schema == "getcodexy.doctor.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .host_readiness.state == "ready" and .errors == [] and ([.component_health[]] | length == 3) and ([.component_health[] | select(.healthy == true and .state == "healthy" and .observed.plugin.version == $version and .observed.runtime.version == $version)] | length == 3)' public-doctor.json >/dev/null

previous_version=${UPGRADE_FROM_VERSION:-}
case "$previous_version" in
'' | *[!0-9.]*)
	echo "previous package version is unavailable" >&2
	exit 1
	;;
esac
test "$previous_version" != "$TARGET_VERSION"
upgrade_code_home="$RUNNER_TEMP/upgrade-codex-home"
mkdir -p "$upgrade_code_home/getcodexy"
printf '[marketplaces.codexy]\nref = "v%s"\n' "$previous_version" >"$upgrade_code_home/config.toml"
touch "$upgrade_code_home/.codexy-public-marketplace-present"
jq -n --arg version "$previous_version" '{selection:["core","github","devtools"],versions:{core:$version,github:$version,devtools:$version}}' >"$upgrade_code_home/.codexy-public-proof.json"
printf '{"components":["core","github","devtools"],"schema":"getcodexy.installed-component-inventory.v1"}\n' >"$upgrade_code_home/getcodexy/installed-components.json"
upgrade_env=(env PATH="$RUNNER_TEMP:$PATH" CODEX_HOME="$upgrade_code_home" CODEXY_RUNTIME_PLATFORM=linux-x86_64 CODEXY_MARKETPLACE_ROOT="$PWD/public-marketplace" FAIL_MARKETPLACE_UPGRADE=1)
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy update --json >public-upgrade.json
jq -e '.schema == "getcodexy.operation-receipt.v1" and .command == "update" and .outcome == "completed" and .errors == [] and (.selection_after | sort == ["core", "devtools", "github"])' public-upgrade.json >/dev/null
"${upgrade_env[@]}" codex plugin list --json >public-upgrade-plugin-inventory.json
jq -e --arg version "$TARGET_VERSION" '(.installed | length == 3) and ([.installed[] | select(.installed == true and .enabled == true and .version == $version)] | length == 3)' public-upgrade-plugin-inventory.json >/dev/null
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy status --json >public-upgrade-status.json
jq -e '.outcome == "completed" and .inventory_consistency == "consistent" and .errors == []' public-upgrade-status.json >/dev/null
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy doctor --json >public-upgrade-doctor.json
jq -e --arg version "$TARGET_VERSION" '.outcome == "completed" and .inventory_consistency == "consistent" and .host_readiness.state == "ready" and .errors == [] and ([.component_health[] | select(.healthy == true and .observed.plugin.version == $version and .observed.runtime.version == $version)] | length == 3)' public-upgrade-doctor.json >/dev/null
