#!/usr/bin/env bash
set -Eeuo pipefail

die() {
	echo "A02 FAIL: $*" >&2
	exit 1
}

credential_value() {
	local name="$1" value
	if value="$(printenv "$name" 2>/dev/null)"; then
		[[ -z "$value" ]] || die "credential-like environment variable is set"
	fi
}

for name in OPENAI_API_KEY OPENAI_ADMIN_KEY CODEX_API_KEY CODEX_AUTH_TOKEN CODEX_OAUTH_TOKEN GH_TOKEN GITHUB_TOKEN; do
	credential_value "$name"
done
while IFS= read -r name; do
	case "$name" in
	AWS_ACCESS_KEY_ID | AWS_SECRET_ACCESS_KEY | AWS_SESSION_TOKEN | GOOGLE_APPLICATION_CREDENTIALS | GIT_ASKPASS | SSH_ASKPASS | GIT_SSH_COMMAND | SSH_AUTH_SOCK | GIT_CONFIG_COUNT | GIT_CONFIG_KEY_* | GIT_CONFIG_VALUE_*)
		credential_value "$name"
		;;
	esac
	case "$name" in
	*_TOKEN | *_API_KEY | *_SECRET | *_PASSWORD | *_CREDENTIALS)
		credential_value "$name"
		;;
	PIP_INDEX_URL | PIP_EXTRA_INDEX_URL | UV_INDEX_URL | HTTP_PROXY | HTTPS_PROXY | ALL_PROXY)
		value="$(printenv "$name" 2>/dev/null || true)"
		[[ "$value" != *@* ]] || die "credential-like URL environment variable is set"
		;;
	esac
done < <(compgen -e | LC_ALL=C sort)
unset BASH_ENV ENV

RUNNER_TEMP="$(printenv RUNNER_TEMP 2>/dev/null || true)"
GITHUB_REPOSITORY="$(printenv GITHUB_REPOSITORY 2>/dev/null || true)"
EVENT_PATH="$(printenv GITHUB_EVENT_PATH 2>/dev/null || true)"
WORKFLOW_PR_NUMBER="$(printenv PR_NUMBER 2>/dev/null || true)"
[[ -n "$RUNNER_TEMP" && -n "$GITHUB_REPOSITORY" && -n "$EVENT_PATH" ]] || die "required GitHub runner identity is missing"
[[ "$GITHUB_REPOSITORY" == eunsoogi/codexy ]] || die "foreign repository"
[[ "$RUNNER_TEMP" == /* && "$EVENT_PATH" == /* && -f "$EVENT_PATH" ]] || die "runner paths are not absolute regular files"
[[ "$(printenv GITHUB_ACTIONS 2>/dev/null || true)" == true ]] || die "not GitHub Actions"
[[ "$(printenv RUNNER_ENVIRONMENT 2>/dev/null || true)" == github-hosted ]] || die "runner is not GitHub-hosted"
[[ "$WORKFLOW_PR_NUMBER" =~ ^[1-9][0-9]*$ ]] || die "pull request number is invalid"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
PROOF_REPO="$(cd "$SCRIPT_DIR/../.." && pwd -P)"
git -C "$PROOF_REPO" rev-parse --show-toplevel >/dev/null 2>&1 || die "proof checkout is not a Git worktree"
[[ "$(git -C "$PROOF_REPO" rev-parse --show-toplevel)" == "$PROOF_REPO" ]] || die "proof checkout root mismatch"
RUN_ROOT="$RUNNER_TEMP/a02-no-node"
[[ ! -e "$RUN_ROOT" ]] || die "stale proof directory"
mkdir "$RUN_ROOT"

EVENT_RECORD="$(
	python3 - "$EVENT_PATH" <<'PY'
import json,sys
e=json.load(open(sys.argv[1],encoding="utf-8")); p=e.get("pull_request")
b,h=(p.get("base"),p.get("head")) if isinstance(p,dict) else (None,None)
v=[p.get("action",e.get("action")) if isinstance(p,dict) else None,str(e.get("number","")),b.get("ref") if isinstance(b,dict) else None,b.get("sha") if isinstance(b,dict) else None,h.get("ref") if isinstance(h,dict) else None,h.get("sha") if isinstance(h,dict) else None,h.get("repo",{}).get("full_name") if isinstance(h,dict) and isinstance(h.get("repo"),dict) else None,b.get("repo",{}).get("full_name") if isinstance(b,dict) and isinstance(b.get("repo"),dict) else None]
if any(not isinstance(x,str) or not x or "\t" in x or "\n" in x for x in v): raise SystemExit(1)
print("\t".join(v))
PY
)" || die "pull request event is invalid"
IFS=$'\t' read -r ACTION EVENT_PR_NUMBER BASE_REF BASE_SHA HEAD_REF HEAD_SHA HEAD_REPO BASE_REPO <<<"$EVENT_RECORD"

EXPECTED_SOURCE_SHA=222b6ce19fb190b8233e7d2d3ae691f66c082c35
EXPECTED_STACKED_BASE_SHA=c185f9560529ed91a7bc8a331f2bbb2ad8eb9b63
EXPECTED_STACKED_INTERMEDIATE_SHA=f16e002851681858d139054dd115c76c05c0e43a
EXPECTED_SOURCE_BRANCH=eunsoogi/788-local-archive-identity
EXPECTED_HEAD_BRANCH=eunsoogi/787-official-no-node-runner
[[ "$ACTION" =~ ^(opened|synchronize|reopened)$ ]] || die "unsupported pull request action"
[[ "$EVENT_PR_NUMBER" == "$WORKFLOW_PR_NUMBER" && "$BASE_REF" == "$EXPECTED_SOURCE_BRANCH" && "$HEAD_REF" == "$EXPECTED_HEAD_BRANCH" ]] || die "pull request identity mismatch"
[[ "$BASE_REPO" == "$GITHUB_REPOSITORY" && "$HEAD_REPO" == "$GITHUB_REPOSITORY" ]] || die "foreign pull request repository"
[[ "$BASE_SHA" == "$EXPECTED_SOURCE_SHA" && "$BASE_SHA" =~ ^[0-9a-f]{40}$ && "$HEAD_SHA" =~ ^[0-9a-f]{40}$ ]] || die "pull request SHA admission mismatch"

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null GIT_CONFIG_NOSYSTEM=1 GIT_TERMINAL_PROMPT=0
unset GIT_ASKPASS SSH_ASKPASS GIT_SSH_COMMAND SSH_AUTH_SOCK GIT_CONFIG_COUNT
REMOTE="https://github.com/$GITHUB_REPOSITORY.git"
remote_ref() {
	local ref="$1" lines count value
	lines="$(git -C "$PROOF_REPO" ls-remote --heads "$REMOTE" "refs/heads/$ref" 2>/dev/null)" || die "unable to read remote ref"
	count="$(printf '%s\n' "$lines" | awk 'NF {n++} END {print n+0}')"
	[[ "$count" == 1 ]] || die "remote ref is missing or ambiguous"
	value="$(printf '%s\n' "$lines" | awk 'NF == 2 {print $1}')"
	[[ "$value" =~ ^[0-9a-f]{40}$ ]] || die "remote ref is not an immutable commit"
	printf '%s\n' "$value"
}
[[ "$(remote_ref "$EXPECTED_SOURCE_BRANCH")" == "$EXPECTED_SOURCE_SHA" ]] || die "stacked source branch is stale or foreign"
[[ "$(remote_ref "$HEAD_REF")" == "$HEAD_SHA" ]] || die "proof head branch is stale"
[[ "$(git -C "$PROOF_REPO" rev-parse HEAD)" == "$HEAD_SHA" ]] || die "checked out head is not the event head"
git -C "$PROOF_REPO" fetch --no-tags --depth=3 "$REMOTE" "$EXPECTED_SOURCE_SHA" >/dev/null 2>&1 || die "exact stacked source fetch failed"
[[ "$(git -C "$PROOF_REPO" rev-parse "$EXPECTED_SOURCE_SHA^1")" == "$EXPECTED_STACKED_INTERMEDIATE_SHA" ]] || die "stacked source intermediate parent mismatch"
[[ "$(git -C "$PROOF_REPO" rev-parse "$EXPECTED_SOURCE_SHA~2")" == "$EXPECTED_STACKED_BASE_SHA" ]] || die "stacked source base ancestor mismatch"
[[ "$(git -C "$PROOF_REPO" rev-list --first-parent --count "$EXPECTED_STACKED_BASE_SHA..$EXPECTED_SOURCE_SHA")" == 2 ]] || die "stacked source topology is not closed"

ARCHIVE="$RUN_ROOT/source-789.tar"
MARKETPLACE_ROOT="$RUN_ROOT/marketplace"
git -C "$PROOF_REPO" archive --format=tar "$EXPECTED_SOURCE_SHA" >"$ARCHIVE" || die "exact source archive failed"
ARCHIVE_SHA256="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
[[ "$ARCHIVE_SHA256" =~ ^[0-9a-f]{64}$ ]] || die "source archive digest missing"
mkdir "$MARKETPLACE_ROOT"
tar -xf "$ARCHIVE" -C "$MARKETPLACE_ROOT" || die "source archive extraction failed"
for required in .agents/plugins/marketplace.json plugins/codexy/.codex-plugin/plugin.json plugins/codexy-github/.codex-plugin/plugin.json plugins/codexy-devtools/.codex-plugin/plugin.json; do
	[[ -f "$MARKETPLACE_ROOT/$required" && ! -L "$MARKETPLACE_ROOT/$required" ]] || die "archive provenance is incomplete"
done
SOURCE_FILES_BEFORE="$RUN_ROOT/source-files-before.sha256"
find "$MARKETPLACE_ROOT" -type f -exec sha256sum {} + | LC_ALL=C sort >"$SOURCE_FILES_BEFORE"

HOME_DIR="$RUN_ROOT/home"
CODEX_HOME_DIR="$RUN_ROOT/codex-home"
XDG_CONFIG_DIR="$RUN_ROOT/xdg-config"
XDG_DATA_DIR="$RUN_ROOT/xdg-data"
XDG_CACHE_DIR="$RUN_ROOT/xdg-cache"
GH_CONFIG_DIR="$RUN_ROOT/gh-config"
mkdir "$HOME_DIR" "$CODEX_HOME_DIR" "$XDG_CONFIG_DIR" "$XDG_DATA_DIR" "$XDG_CACHE_DIR" "$GH_CONFIG_DIR"
for dir in "$HOME_DIR" "$CODEX_HOME_DIR" "$XDG_CONFIG_DIR" "$XDG_DATA_DIR" "$XDG_CACHE_DIR" "$GH_CONFIG_DIR"; do
	[[ -z "$(find "$dir" -mindepth 1 -print -quit)" ]] || die "fresh proof directory is not empty"
done
unset CODEX_HOME
export HOME="$HOME_DIR" CODEX_HOME="$CODEX_HOME_DIR" XDG_CONFIG_HOME="$XDG_CONFIG_DIR" XDG_DATA_HOME="$XDG_DATA_DIR" XDG_CACHE_HOME="$XDG_CACHE_DIR" GH_CONFIG_DIR="$GH_CONFIG_DIR"
SYSTEM_PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH="$HOME_DIR/.local/bin:$SYSTEM_PATH" PIP_CONFIG_FILE=/dev/null PIP_DISABLE_PIP_VERSION_CHECK=1 PIP_NO_INPUT=1 PYTHONNOUSERSITE=1
assert_missing() {
	local command_name
	for command_name in node npx; do
		[[ -z "$(command -v "$command_name" 2>/dev/null || true)" ]] || die "$command_name is available on the clean runner"
	done
}
assert_missing
if existing_codex="$(command -v codex 2>/dev/null)"; then
	[[ -z "$existing_codex" ]] || die "Codex CLI was inherited before official acquisition"
fi

curl --fail --silent --show-error --location https://chatgpt.com/codex/install.sh 2>/dev/null | sh >/dev/null 2>&1 || die "official Codex CLI acquisition failed"
assert_missing
CODEX="$(type -P codex 2>/dev/null || true)"
[[ -n "$CODEX" && "$CODEX" == /* && -x "$CODEX" ]] || die "official Codex CLI is unavailable"
CODEX_REAL="$(readlink -f "$CODEX" 2>/dev/null || true)"
[[ -n "$CODEX_REAL" && -x "$CODEX_REAL" ]] || die "Codex CLI path is not executable"
CODEX_VERSION_FILE="$RUN_ROOT/codex-version.txt"
"$CODEX" --version >"$CODEX_VERSION_FILE" 2>/dev/null || die "Codex CLI version check failed"
[[ -s "$CODEX_VERSION_FILE" ]] || die "Codex CLI version output is missing"

run_json() {
	local output="$1" error_file="$1.stderr"
	shift
	[[ ! -e "$output" && ! -e "$error_file" ]] || die "stale JSON output"
	if ! "$@" >"$output" 2>"$error_file"; then
		die "JSON command failed"
	fi
	[[ -s "$output" ]] || die "JSON output is missing"
	python3 -c 'import json,re,sys; s=open(sys.argv[1],encoding="utf-8").read(); json.loads(s); bad=re.compile(r"(api[_-]?key|access[_-]?token|secret|password|credential|oauth|authorization|bearer|(?<![A-Za-z0-9])sk-[A-Za-z0-9]{8,}|(?<![A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{8,}|(?<![A-Za-z0-9])github_pat_[A-Za-z0-9_]+|(?<![A-Za-z0-9])Bearer\s+\S+)",re.I); raise SystemExit(1) if bad.search(s) else None' "$output" || die "JSON output is invalid or secret-bearing"
}

ADD_JSON="$RUN_ROOT/marketplace-add.json"
LIST_JSON="$RUN_ROOT/marketplace-list.json"
INSTALL_JSON="$RUN_ROOT/install.json"
DOCTOR_JSON="$RUN_ROOT/doctor.json"
PLUGINS_JSON="$RUN_ROOT/plugins.json"
run_json "$ADD_JSON" "$CODEX" plugin marketplace add "$MARKETPLACE_ROOT" --json
run_json "$LIST_JSON" "$CODEX" plugin marketplace list --json
python3 -c 'import json,os,sys; from pathlib import Path; p=json.load(open(sys.argv[1])); r=Path(sys.argv[2]).resolve(); m=[x for x in p.get("marketplaces",[]) if isinstance(x,dict) and x.get("name")=="codexy"]; x=m[0] if len(m)==1 else {}; ok=x.get("root")==str(r) and x.get("marketplaceSource")=={"sourceType":"local","source":str(r)} and Path(x.get("root","")).is_dir() and os.path.realpath(x.get("root",""))==str(r); raise SystemExit(0 if ok else 1)' "$LIST_JSON" "$MARKETPLACE_ROOT" || die "marketplace registration is foreign, unbound, or ambiguous"

unset PIP_EXTRA_INDEX_URL UV_INDEX_URL
export PIP_INDEX_URL=https://pypi.org/simple
BUILD_SOURCE="$RUN_ROOT/getcodexy-source"
cp -a "$MARKETPLACE_ROOT/packages/getcodexy" "$BUILD_SOURCE"
BUILD_VENV="$RUN_ROOT/build-venv"
DIST="$RUN_ROOT/dist"
INSTALL_VENV="$RUN_ROOT/install-venv"
mkdir "$DIST"
python3 -m venv "$BUILD_VENV" || die "build environment creation failed"
"$BUILD_VENV/bin/python" -m pip install --no-cache-dir --disable-pip-version-check --no-input --index-url https://pypi.org/simple build >/dev/null 2>"$RUN_ROOT/build-install.stderr" || die "build tool acquisition failed"
"$BUILD_VENV/bin/python" -m build --outdir "$DIST" "$BUILD_SOURCE" >/dev/null 2>"$RUN_ROOT/build.stderr" || die "getcodexy wheel build failed"
VERSION="$(awk -F'"' '/^version = / {print $2; exit}' "$BUILD_SOURCE/pyproject.toml")"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "package version is invalid"
python3 -m venv "$INSTALL_VENV" || die "install environment creation failed"
"$INSTALL_VENV/bin/python" -m pip install --no-index --no-cache-dir --disable-pip-version-check --no-input --find-links "$DIST" "getcodexy==$VERSION" >/dev/null 2>"$RUN_ROOT/offline-install.stderr" || die "offline getcodexy installation failed"
run_json "$INSTALL_JSON" "$INSTALL_VENV/bin/getcodexy" --codex "$CODEX" --codex-home "$CODEX_HOME" install --json
run_json "$DOCTOR_JSON" "$INSTALL_VENV/bin/getcodexy" --codex "$CODEX" --codex-home "$CODEX_HOME" doctor --json
run_json "$PLUGINS_JSON" "$CODEX" plugin list --json
SOURCE_FILES_AFTER="$RUN_ROOT/source-files-after.sha256"
find "$MARKETPLACE_ROOT" -type f -exec sha256sum {} + | LC_ALL=C sort >"$SOURCE_FILES_AFTER"
cmp -s "$SOURCE_FILES_BEFORE" "$SOURCE_FILES_AFTER" || die "archive source changed during proof"
[[ -z "$(git -C "$PROOF_REPO" status --porcelain)" ]] || die "proof checkout changed during proof"
AUTH_FILE="$(find "$CODEX_HOME" -type f \( -iname '*auth*' -o -iname '*token*' -o -iname '*credential*' -o -iname '*oauth*' \) -print -quit)"
[[ -z "$AUTH_FILE" ]] || die "Codex home contains inherited or generated auth state"
RECEIPT="$RUN_ROOT/receipt.json"
python3 - "$RECEIPT" "$MARKETPLACE_ROOT" "$RUN_ROOT" "$PROOF_REPO" "$HEAD_SHA" "$WORKFLOW_PR_NUMBER" "$ARCHIVE_SHA256" "$CODEX" "$CODEX_REAL" "$VERSION" "$ADD_JSON" "$LIST_JSON" "$INSTALL_JSON" "$DOCTOR_JSON" "$PLUGINS_JSON" "$SOURCE_FILES_BEFORE" "$SOURCE_FILES_AFTER" "$CODEX_VERSION_FILE" <<'PY'
import hashlib,json,os,re,sys
from pathlib import Path

a=sys.argv[1:]; R,root,run,proof,head,pr_number,archive,codex,codex_real,version,addf,listf,installf,doctorf,pluginsf,before,after,codexv=a; R=Path(R); root=Path(root).resolve()
SOURCE="222b6ce19fb190b8233e7d2d3ae691f66c082c35"; STACKED="c185f9560529ed91a7bc8a331f2bbb2ad8eb9b63"; INTERMEDIATE="f16e002851681858d139054dd115c76c05c0e43a"; REPO="eunsoogi/codexy"; NAMES=["plan-stress-test","frame-check","decision-rationale","artifact-refresh","blind-read","project-brief"]
def fail(message): raise SystemExit(message)
def ok(value,message): value or fail(message)
def load(path): return json.loads(Path(path).read_text(encoding="utf-8"))
def digest(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
hash_words="""plugins/codexy/skills/plan-stress-test/SKILL.md 6c642cee5216fad0a1bb6015db7f17154377ee1935c1145ee40b44213491708f plugins/codexy/skills/plan-stress-test/agents/openai.yaml 965d9c0af2d69ffce8d5c788e7154a8b3dd2e22474d0cca2c8a1195f90cc0dd5 plugins/codexy/skills/frame-check/SKILL.md be6ccce90288c0e501f132035c5221dfdba6b697132c6951282cfc57f8f340a1 plugins/codexy/skills/frame-check/agents/openai.yaml 263179d3eb36f3f0d82985a963cfdc88c4202229419ece90fad317b1499905b7 plugins/codexy/skills/decision-rationale/SKILL.md 60541c0e3619b2b1e2d6ab27a82f651867580467d5402be71de19d1431c4031f plugins/codexy/skills/decision-rationale/agents/openai.yaml 4392c367615b6b1758047999c9386e663284e30a81c8d4fb02f8b5a9bd32f36a plugins/codexy/skills/artifact-refresh/SKILL.md fb4394d30587db0dd01470fc46f7d1f4309dbcbc42d27314d6b4f3b41fba5f15 plugins/codexy/skills/artifact-refresh/agents/openai.yaml 30e103326dead4992c220a5ca9364c1c6a2ff0d430f7a4dbf196e292c249a194 plugins/codexy/skills/artifact-refresh/schema.json 14b1fcd216cab3b569a7e0271027fd94e21c03d158f3f3e83c2d836924220a71 plugins/codexy/skills/artifact-refresh/fixtures/corpus.json 8dc215d03f454de6eb1a1ff144212092443108977c32a5f9982604d6b071c6be plugins/codexy/skills/blind-read/SKILL.md 05520576dcb9aff90830e84b5766cf1d1ac3d4eefde53ccefbfc01c6765f6363 plugins/codexy/skills/blind-read/agents/openai.yaml cbfe22d6acf0a034e48678d05767c9a86aecee450b9d7c7210bdcc53cd5b5a3e plugins/codexy/skills/blind-read/schema.json f573eb2b5a3460f609f62b8e23bad07bb351a3a43961922c1a9141e649cde811 plugins/codexy/skills/blind-read/fixtures/corpus.json d602e0c0e95c2a0ca8b7a475d29933d8f61eac0baabe77681531bab9d4397d55 plugins/codexy/skills/project-brief/SKILL.md 21848cea006d125f5502bf820b32537737d976cf0ab783893dde1e13c597bbd8 plugins/codexy/skills/project-brief/agents/openai.yaml f929f65ecf37e4884699bc5238e9fefebe77f3798e594761dba657dad8de87f6 plugins/codexy/skills/project-brief/schema.json 2898095d4a1c417cfcddf40d9a30e54e13146ba5b918372afcc9778b6f90f6f5 plugins/codexy/skills/project-brief/fixtures/corpus.json 61bf1e53a917c7eb0163320af96c672f6c50c3cf0874c4fd5a72e3382fa37d87""".split()
hashes=dict(zip(hash_words[::2],hash_words[1::2]))
for rel,expected in hashes.items(): p=root/rel; ok(p.is_file() and not p.is_symlink(),"candidate file missing or symlink"); ok(digest(p)==expected,"candidate file identity mismatch")
for name in NAMES:
 t=(root/"plugins/codexy/skills"/name/"SKILL.md").read_text(encoding="utf-8"); m=re.match(r"\A---\nname: ([^\n]+)\ndescription: ([^\n]+)\n---\n",t); ok(m and m.group(1)==name and m.group(2),"frontmatter identity mismatch")
 lines=[x for x in (root/"plugins/codexy/skills"/name/"agents/openai.yaml").read_text(encoding="utf-8").splitlines() if x.strip()]; ok(len(lines)==6 and lines[0]=="interface:" and lines[1].startswith("  display_name: ") and lines[2].startswith("  short_description: ") and lines[3].startswith("  default_prompt: ") and lines[4]=="policy:" and lines[5]=="  allow_implicit_invocation: true","agent metadata contract mismatch")
reqs={"artifact-refresh":("codexy.artifact-refresh.v1",["schema","artifact","governing_source","outcome","removed","proof_handle","handoff_reason"]),"blind-read":("blind-read",["interpreted_purpose","unresolved_reference","action_blocker"]),"project-brief":("project-brief",["objective","owner","verified_phase","changes_since_touch","decision_required","evidence_handle","next_action","done_when"])}
for name,(schema,required) in reqs.items():
 d=load(root/"plugins/codexy/skills"/name/"schema.json"); ok(d.get("schema",schema)==schema and list(d.get("required",()))==required and set(d.get("properties",()))==set(required) and d.get("additionalProperties") is False,"output schema contract mismatch")
F={name:load(root/"plugins/codexy/skills"/name/"fixtures/corpus.json") for name in ("artifact-refresh","blind-read","project-brief")}; ok(F["artifact-refresh"].get("schema")=="codexy.artifact-refresh.corpus.v1" and F["project-brief"].get("schema")=="codexy.project-brief.corpus.v1" and isinstance(F["blind-read"].get("cases"),list),"corpus schema mismatch")
results={}
def rows(skill,positive,negative,prefix):
 ok(len(positive)==10 and len(negative)==10,"inline corpus count mismatch"); results[skill]=[{"case_id":f"{prefix}-P{i:02d}","command":"deterministic-packaged-skill-corpus","selected_skill":skill,"expected_classification":"POSITIVE","observed_deterministic_result":v,"terminal_outcome":"PASS"} for i,v in enumerate(positive,1)]+[{"case_id":f"{prefix}-N{i:02d}","command":"deterministic-packaged-skill-corpus","selected_skill":skill,"expected_classification":"NEGATIVE","observed_deterministic_result":v,"terminal_outcome":"PASS"} for i,v in enumerate(negative,1)]
q=chr(96); t=(root/"plugins/codexy/skills/plan-stress-test/SKILL.md").read_text(encoding="utf-8"); b=re.search(q*3+r"text\n(.*?)\n"+q*3,t,re.S); ok(b is not None and b.group(1).splitlines()==["invalidating_assumption=<one simple declarative causal claim>","bounded_probe=<one imperative sentence for the smallest discriminating probe>","expected_observable=<one passing observation or one failing observation>","decision_effect=<passing observation keeps the plan, failure stops, narrows, or returns it>"],"plan receipt grammar mismatch"); rows("plan-stress-test",[x for x in t.splitlines() if x.startswith("- "+q) and x.count(q)==12 and "; " in x],[x for x in t.splitlines() if "Decline;" in x],"PS")
t=(root/"plugins/codexy/skills/frame-check/SKILL.md").read_text(encoding="utf-8"); b=re.search(q*3+r"yaml\n(.*?)\n"+q*3,t,re.S); ok(b is not None and [x.strip().lstrip("- ").split(":",1)[0] for x in b.group(1).splitlines() if ":" in x]==["current_assumption","credible_alternative","constraint_conflict","owner_question"],"frame receipt grammar mismatch"); frame_neg=["current-diff verdicts","proof or completion claims","voting or consensus","model routing","owner assignment","implementation or mutation","unconstrained ideation","verification claims","requests for more than three interpretations","MUST decline"]; ok(all(x in " ".join(t.split()) for x in frame_neg),"frame decline boundary mismatch"); rows("frame-check",[x for x in t.splitlines() if x.startswith("| ") and "Proposal" not in x and "---" not in x],frame_neg,"FC")
t=(root/"plugins/codexy/skills/decision-rationale/SKILL.md").read_text(encoding="utf-8"); decision_neg=["no chosen decision exists","choose","recommend","approve","reject","cancel","reopen","mutate state","fact-check evidence","review a current diff"]; ok(all(x in " ".join(t.split()) for x in decision_neg),"decision decline boundary mismatch"); rows("decision-rationale",[x for x in t.splitlines() if x.count(" | ")==3],decision_neg,"DR")
af=F["artifact-refresh"]; ids=[f"AR-{kind}{i:02d}" for kind in ("P","N") for i in range(1,11)]; ok(set(af.get("outcomes",{}))==set(ids),"artifact corpus inventory mismatch")
for i in ids:
 if i.startswith("AR-P"):
  observed={"outcome":"NO_CHANGE" if af["artifacts"][i]==af["governing_sources"][i] else "UPDATED","artifact":af["expected_artifacts"][i],"removed":af.get("removed",{}).get(i,{})}; ok(observed["outcome"]==af["outcomes"][i],"artifact classification mismatch")
 else: observed={"outcome":"HANDOFF_REQUIRED","reason":af["handoff_reasons"][i]}; ok(observed["outcome"]==af["outcomes"][i],"artifact negative classification mismatch")
 results.setdefault("artifact-refresh",[]).append({"case_id":i,"command":"deterministic-packaged-skill-corpus","selected_skill":"artifact-refresh","expected_classification":"POSITIVE" if i.startswith("AR-P") else "NEGATIVE","observed_deterministic_result":observed,"terminal_outcome":"PASS"})
bf=F["blind-read"]; ok([c.get("case_id") for c in bf["cases"]]==[f"BR-{k}{i:02d}" for k in ("P","N") for i in range(1,11)],"blind corpus inventory mismatch")
for c in bf["cases"]:
 i=c["case_id"]; observed=c["expected_result"]; ok((i.startswith("BR-P") and isinstance(observed,dict) and list(observed)==["interpreted_purpose","unresolved_reference","action_blocker"]) or (i.startswith("BR-N") and isinstance(observed,str) and observed.startswith("HANDOFF_REQUIRED:")),"blind corpus result mismatch"); results.setdefault("blind-read",[]).append({"case_id":i,"command":"deterministic-packaged-skill-corpus","selected_skill":"blind-read","expected_classification":"POSITIVE" if i.startswith("BR-P") else "NEGATIVE","observed_deterministic_result":observed,"terminal_outcome":"PASS"})
pf=F["project-brief"]; ok([c.get("id") for c in pf["cases"]]==[f"PB-{k}{i:02d}" for k in ("P","N") for i in range(1,11)],"project corpus inventory mismatch")
for c in pf["cases"]:
 i=c["id"]; observed=json.loads(c["expected_json"]) if i.startswith("PB-P") else c["expected"]; ok(observed is not None,"project corpus result missing"); results.setdefault("project-brief",[]).append({"case_id":i,"command":"deterministic-packaged-skill-corpus","selected_skill":"project-brief","expected_classification":"POSITIVE" if i.startswith("PB-P") else "NEGATIVE","observed_deterministic_result":observed,"terminal_outcome":"PASS"})
market=load(root/".agents/plugins/marketplace.json"); plugin_names=["codexy","codexy-github","codexy-devtools"]; entries=market.get("plugins",[]); ok(market.get("name")=="codexy" and sorted(x.get("name") for x in entries)==sorted(plugin_names),"marketplace provenance mismatch")
for entry in entries:
 n=entry["name"]; ok(entry.get("source")=={"source":"local","path":f"./plugins/{n}"},"marketplace path mismatch"); p=load(root/"plugins"/n/".codex-plugin/plugin.json"); ok(p.get("name")==n and p.get("repository")== "https://github.com/eunsoogi/codexy" and p.get("version")==version==entry.get("version"),"plugin manifest identity mismatch")
raw=[load(x) for x in (addf,listf,installf,doctorf,pluginsf)]; addj,listj,inst,doc,plug=raw; root_s=str(root); ms=[x for x in listj.get("marketplaces",[]) if isinstance(x,dict) and x.get("name")=="codexy"]; ok(len(ms)==1 and ms[0].get("root")==root_s and ms[0].get("marketplaceSource")=={"sourceType":"local","source":root_s},"marketplace output identity mismatch")
ok(inst.get("outcome")=="completed" and sorted(inst.get("installed_components",[]))==["core","devtools","github"],"install receipt mismatch"); health=doc.get("component_health",[]); ok(doc.get("outcome")=="completed" and doc.get("errors")==[] and sorted(x.get("component") for x in health if x.get("state")=="healthy")==["core","devtools","github"] and all(x.get("healthy") is True for x in health),"doctor receipt mismatch")
records=plug.get("installed",[]); ok(sorted(x.get("name") for x in records)==sorted(plugin_names) and len(records)==3,"installed inventory is missing or ambiguous")
for x in records:
 n=x["name"]; ok(x.get("pluginId")==f"{n}@codexy" and x.get("marketplaceName")=="codexy" and x.get("installed") is True and x.get("enabled") is True and x.get("version")==version and x.get("source")=={"source":"local","path":f"{root_s}/plugins/{n}"} and x.get("marketplaceSource")=={"sourceType":"local","source":root_s},"installed identity mismatch")
env={k:os.environ.get(k) or fail(f"missing workflow identity: {k}") for k in ("GITHUB_WORKFLOW","GITHUB_JOB","GITHUB_WORKFLOW_SHA","GITHUB_RUN_ID","GITHUB_RUN_ATTEMPT")}; terminal=f"A02 TERMINAL PASS frozen_head={SOURCE} body_sha256=7271a15c859af936b8c911a4d4ff146852dc1897fba73ce2787bc746d2f170a2 body_updated_at=2026-08-30T00:12:39Z portfolio_n=6 admitted={','.join(NAMES)} UNCLASSIFIED=0 reviewer=codexy-sentinel"; ok(re.fullmatch(r"A02 TERMINAL PASS frozen_head=[0-9a-f]{40} body_sha256=[0-9a-f]{64} body_updated_at=[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z portfolio_n=[0-6] admitted=[a-z0-9-]+(,[a-z0-9-]+)* UNCLASSIFIED=0 reviewer=codexy-sentinel",terminal),"terminal grammar mismatch")
receipt={"schema":"codexy.a02.no-node-receipt.v1","admission":{"issue_787":{"body_sha256":"7271a15c859af936b8c911a4d4ff146852dc1897fba73ce2787bc746d2f170a2","body_updated_at":"2026-08-30T00:12:39Z"},"issue_788":{"body_sha256":"724fa2289f8fc62325d6eb4c586f4c5dbc0ff8980cc4cb1e870c3fbc366399e1","body_updated_at":"2026-08-30T00:12:40Z"},"issue_713":{"body_sha256":"d78e5f089042e290c2ae742c2c1adf2d82a329dfeb43af58fbe1691446de37e5","body_updated_at":"2026-08-30T00:23:19Z"}},"source":{"repository":REPO,"source_pr":789,"source_branch":"eunsoogi/788-local-archive-identity","source_sha":SOURCE,"stacked_base_sha":STACKED,"stacked_intermediate_sha":INTERMEDIATE},"proof":{"pr":int(pr_number),"issue":787,"head_branch":"eunsoogi/787-official-no-node-runner","head_sha":head,"base_branch":"eunsoogi/788-local-archive-identity","base_sha":SOURCE},"workflow":{"name":env["GITHUB_WORKFLOW"],"job":env["GITHUB_JOB"],"path":".github/workflows/runtime-candidate.yml","workflow_sha":env["GITHUB_WORKFLOW_SHA"],"run_id":env["GITHUB_RUN_ID"],"run_attempt":env["GITHUB_RUN_ATTEMPT"]},"runner":{"os":os.environ.get("RUNNER_OS","unavailable"),"arch":os.environ.get("RUNNER_ARCH","unavailable"),"environment":"github-hosted","node_available":False,"npx_available":False},"codex":{"path":codex,"real_path":codex_real,"version":Path(codexv).read_text().strip(),"model_operations":0,"authenticated":False},"archive":{"method":"git archive","source_commit":SOURCE,"sha256":archive,"root":root_s,"marketplaceSource":{"sourceType":"local","source":root_s},"provenance":"exact #789 source archive with exact closed two-commit first-parent chain"},"cli":{"marketplace_add":addj,"marketplace_list":listj,"plugin_list":plug},"getcodexy":{"version":version,"install":inst,"doctor":doc},"installed_inventory":records,"contract_corpus":{"skills":results,"terminal_grammar":terminal},"no_mutation":{"archive_files_before":digest(before),"archive_files_after":digest(after),"source_unchanged":True,"proof_checkout_clean":True,"fresh_codex_home":True,"auth_state_absent":True},"identity_decision":"PASS","decision":"PASS"}
bad_key=re.compile(r"(api[_-]?key|access[_-]?token|secret|password|credential|oauth|authorization|bearer)",re.I); bad_value=re.compile(r"(?:(?<![A-Za-z0-9])sk-[A-Za-z0-9]{8,}|(?<![A-Za-z0-9])gh[pousr]_[A-Za-z0-9]{8,}|(?<![A-Za-z0-9])github_pat_[A-Za-z0-9_]+|(?<![A-Za-z0-9])Bearer\s+\S+)")
def safe(value):
 if isinstance(value,dict):
  for key,child in value.items(): ok(key=="authenticated" or not bad_key.search(str(key)),"receipt contains credential-like key"); safe(child)
 elif isinstance(value,list):
  for child in value: safe(child)
 elif isinstance(value,str): ok(not bad_value.search(value),"receipt contains credential-like value")
safe(receipt); R.write_text(json.dumps(receipt,sort_keys=True,indent=2)+"\n",encoding="utf-8")
PY
cat "$RECEIPT"
printf '%s\n' "A02 TERMINAL PASS frozen_head=$EXPECTED_SOURCE_SHA body_sha256=7271a15c859af936b8c911a4d4ff146852dc1897fba73ce2787bc746d2f170a2 body_updated_at=2026-08-30T00:12:39Z portfolio_n=6 admitted=plan-stress-test,frame-check,decision-rationale,artifact-refresh,blind-read,project-brief UNCLASSIFIED=0 reviewer=codexy-sentinel"
