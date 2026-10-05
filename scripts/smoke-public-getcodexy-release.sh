#!/usr/bin/env bash
set -euo pipefail

report_smoke_failure() {
	local status="$1" command="$2" receipt
	printf 'Public smoke failed (exit %s): %s\n' "$status" "$command" >&2
	for receipt in public-install.json public-github-install.log public-status.json \
		public-doctor.json public-upgrade.json public-upgrade-github-install.log \
		public-upgrade-status.json public-upgrade-doctor.json; do
		if [[ -f "$receipt" ]]; then
			printf '%s\n' "--- $receipt ---" >&2
			cat -- "$receipt" >&2
		fi
	done
	exit "$status"
}
trap 'report_smoke_failure "$?" "$BASH_COMMAND"' ERR

wait_for_public_getcodexy_simple_index() {
	local attempt=0 content_type index_json="$RUNNER_TEMP/public-getcodexy-simple-index.json"
	local package_type artifact_url digest filename
	local wheel_filename='' wheel_sha256='' sdist_filename='' sdist_sha256=''
	if [[ ! -f public-package-artifacts.tsv ]]; then
		echo "verified public package artifact list is missing" >&2
		return 1
	fi
	while IFS=$'\t' read -r package_type artifact_url digest filename; do
		case "$package_type" in
		bdist_wheel)
			if [[ -n "$wheel_filename" || -z "$artifact_url" || ! "$digest" =~ ^[a-f0-9]{64}$ || "$filename" != "getcodexy-${TARGET_VERSION}-"*.whl ]]; then
				echo "verified public wheel identity is malformed" >&2
				return 1
			fi
			wheel_filename=$filename
			wheel_sha256=$digest
			;;
		sdist)
			if [[ -n "$sdist_filename" || -z "$artifact_url" || ! "$digest" =~ ^[a-f0-9]{64}$ || "$filename" != "getcodexy-${TARGET_VERSION}.tar.gz" ]]; then
				echo "verified public sdist identity is malformed" >&2
				return 1
			fi
			sdist_filename=$filename
			sdist_sha256=$digest
			;;
		*)
			echo "verified public package artifact list has an unknown distribution type" >&2
			return 1
			;;
		esac
	done <public-package-artifacts.tsv
	if [[ -z "$wheel_filename" || -z "$sdist_filename" ]]; then
		echo "verified public package artifact list must contain one wheel and one sdist" >&2
		return 1
	fi

	# Eighteen five-second requests and seventeen ten-second sleeps cap readiness at 260 seconds.
	while [[ "$attempt" -lt 18 ]]; do
		attempt=$((attempt + 1))
		if content_type=$(curl --fail --silent --show-error --location \
			--connect-timeout 5 --max-time 5 \
			--header 'Accept: application/vnd.pypi.simple.v1+json' \
			--output "$index_json" --write-out '%{content_type}' \
			https://pypi.org/simple/getcodexy/); then
			content_type=$(printf '%s' "${content_type%%;*}" | tr '[:upper:]' '[:lower:]')
			if [[ "$content_type" != 'application/vnd.pypi.simple.v1+json' ]]; then
				echo "PyPI Simple Index returned an unexpected content type" >&2
				return 1
			fi
			if ! jq -e '
				if type == "object" and (.meta | type == "object") and .name == "getcodexy" and (.files | type == "array") then
					(.meta["api-version"] | type == "string" and test("^1\\.[0-9]+$")) and
					all(.files[]; type == "object" and (.filename | type == "string" and length > 0) and (.url | type == "string" and length > 0) and (.hashes | type == "object"))
				else false end
			' "$index_json" >/dev/null; then
				echo "PyPI Simple Index returned malformed metadata" >&2
				return 1
			fi
			if jq -e --arg wheel "$wheel_filename" --arg wheel_sha "$wheel_sha256" \
				--arg sdist "$sdist_filename" --arg sdist_sha "$sdist_sha256" '
				(
					any(.files[]; .filename == $wheel) and
					any(.files[]; .filename == $wheel and .hashes.sha256 != $wheel_sha)
				) or (
					any(.files[]; .filename == $sdist) and
					any(.files[]; .filename == $sdist and .hashes.sha256 != $sdist_sha)
				)
			' "$index_json" >/dev/null; then
				echo "PyPI Simple Index SHA-256 differs from verified PyPI JSON" >&2
				return 1
			fi
			if jq -e --arg wheel "$wheel_filename" --arg wheel_sha "$wheel_sha256" \
				--arg sdist "$sdist_filename" --arg sdist_sha "$sdist_sha256" '
				any(.files[]; .filename == $wheel and .hashes.sha256 == $wheel_sha) and
				any(.files[]; .filename == $sdist and .hashes.sha256 == $sdist_sha)
			' "$index_json" >/dev/null; then
				return 0
			fi
		fi
		if [[ "$attempt" -lt 18 ]]; then
			sleep 10
		fi
	done
	echo "PyPI Simple Index did not expose both exact distributions after ${attempt} checks" >&2
	return 1
}

: "${TARGET_VERSION:?}"
: "${RUNNER_TEMP:?}"

previous_version=${UPGRADE_FROM_VERSION:-}
case "$previous_version" in
'' | *[!0-9.]*)
	echo "previous package version is unavailable" >&2
	exit 1
	;;
esac
test "$previous_version" != "$TARGET_VERSION"

python -m venv public-bootstrap
# Install the released bootstrap in isolation before exercising install/update commands.
public-bootstrap/bin/python -m pip install --no-cache-dir uv
export PATH="$PWD/public-bootstrap/bin:$PATH"
export UV_CACHE_DIR="$RUNNER_TEMP/public-smoke-uv-cache"
if [[ -n "${GETCODEXY_DIST:-}" ]]; then
	public-bootstrap/bin/python -m pip install --no-cache-dir --no-index \
		--find-links "$GETCODEXY_DIST" "getcodexy==${TARGET_VERSION}"
	# Registered MCP commands invoke uvx from each plugin directory.
	export UV_NO_INDEX=1
	UV_FIND_LINKS="$(cd "$GETCODEXY_DIST" && pwd)"
	export UV_FIND_LINKS
else
	# Wait for the already-verified wheel and sdist; the local pre-publication path must not use PyPI.
	wait_for_public_getcodexy_simple_index
	public-bootstrap/bin/python -m pip install --no-cache-dir \
		--index-url https://pypi.org/simple "getcodexy==${TARGET_VERSION}"
fi
public_inspect_root=${PUBLIC_INSPECT_ROOT:-public-inspect}
public_bundle_archive=${PUBLIC_BUNDLE_ARCHIVE:-public-bundle.tar.gz}
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/plugins/codexy-devtools" -- --help
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/$public_inspect_root/plugins/codexy-devtools" -- --help
mkdir -p public-marketplace
# Create a fresh local marketplace repository to exercise public bundle discovery end to end.
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
# The first install must observe a genuinely empty Codex home.
mkdir "$public_code_home"
test -z "$(find "$public_code_home" -mindepth 1 -print -quit)"
proof_env=(env PATH="$RUNNER_TEMP:$PATH" CODEX_HOME="$public_code_home" CODEXY_RUNTIME_PLATFORM=linux-x86_64 CODEXY_MARKETPLACE_ROOT="$PWD/public-marketplace")
"${proof_env[@]}" public-bootstrap/bin/getcodexy install --json >public-install.json
jq -e '.schema == "getcodexy.operation-receipt.v1" and .outcome == "completed" and .errors == [] and (.selection_after | sort == ["core", "devtools", "github"])' public-install.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/codexy-github-install \
	--codex "$RUNNER_TEMP/codex" \
	--codex-home "$public_code_home" >public-github-install.log
"${proof_env[@]}" codex plugin list --json >public-plugin-inventory.json
jq -e --arg version "$TARGET_VERSION" '(.installed | length == 3) and ([.installed[] | select(.installed == true and .enabled == true and .version == $version)] | length == 3) and ([.installed[].name] | sort == ["codexy", "codexy-devtools", "codexy-github"])' public-plugin-inventory.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy status --json >public-status.json
jq -e '.schema == "getcodexy.status.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .errors == [] and ([.installed_components[]] | sort == ["core", "devtools", "github"])' public-status.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy doctor --json >public-doctor.json
jq -e --arg version "$TARGET_VERSION" '.schema == "getcodexy.doctor.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .host_readiness.state == "ready" and .errors == [] and ([.component_health[]] | length == 3) and ([.component_health[] | select(.healthy == true and .state == "healthy" and .observed.plugin.version == $version and .observed.runtime.version == $version)] | length == 3)' public-doctor.json >/dev/null

upgrade_code_home="$RUNNER_TEMP/upgrade-codex-home"
# Seed only the previous-version marketplace and selection state for the upgrade path.
mkdir -p "$upgrade_code_home/getcodexy"
printf '[marketplaces.codexy]\nref = "v%s"\n' "$previous_version" >"$upgrade_code_home/config.toml"
touch "$upgrade_code_home/.codexy-public-marketplace-present"
jq -n --arg version "$previous_version" '{selection:["core","github","devtools"],versions:{core:$version,github:$version,devtools:$version}}' >"$upgrade_code_home/.codexy-public-proof.json"
printf '{"components":["core","github","devtools"],"schema":"getcodexy.installed-component-inventory.v1"}\n' >"$upgrade_code_home/getcodexy/installed-components.json"
upgrade_env=(env PATH="$RUNNER_TEMP:$PATH" CODEX_HOME="$upgrade_code_home" CODEXY_RUNTIME_PLATFORM=linux-x86_64 CODEXY_MARKETPLACE_ROOT="$PWD/public-marketplace" FAIL_MARKETPLACE_UPGRADE=1)
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy update --json >public-upgrade.json
jq -e '.schema == "getcodexy.operation-receipt.v1" and .command == "update" and .outcome == "completed" and .errors == [] and (.selection_after | sort == ["core", "devtools", "github"])' public-upgrade.json >/dev/null
"${upgrade_env[@]}" public-bootstrap/bin/codexy-github-install \
	--codex "$RUNNER_TEMP/codex" \
	--codex-home "$upgrade_code_home" >public-upgrade-github-install.log
"${upgrade_env[@]}" codex plugin list --json >public-upgrade-plugin-inventory.json
jq -e --arg version "$TARGET_VERSION" '(.installed | length == 3) and ([.installed[] | select(.installed == true and .enabled == true and .version == $version)] | length == 3)' public-upgrade-plugin-inventory.json >/dev/null
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy status --json >public-upgrade-status.json
jq -e '.outcome == "completed" and .inventory_consistency == "consistent" and .errors == []' public-upgrade-status.json >/dev/null
"${upgrade_env[@]}" public-bootstrap/bin/getcodexy doctor --json >public-upgrade-doctor.json
jq -e --arg version "$TARGET_VERSION" '.outcome == "completed" and .inventory_consistency == "consistent" and .host_readiness.state == "ready" and .errors == [] and ([.component_health[] | select(.healthy == true and .observed.plugin.version == $version and .observed.runtime.version == $version)] | length == 3)' public-upgrade-doctor.json >/dev/null
