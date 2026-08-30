#!/usr/bin/env bash
set -Eeuo pipefail

exec python3 - "$@" <<'PY'
import copy, hashlib, io, json, os, re, subprocess, sys, tarfile, tempfile
from pathlib import Path, PurePosixPath
from urllib.parse import urlsplit
class Failure(Exception): pass
PLATFORM = "linux-x86_64"
MAX_ARCHIVE = 64 * 1024 * 1024
MAX_MEMBER = 32 * 1024 * 1024
SEMVER = re.compile(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
def need(value, message):
    if not value: raise Failure(message)
def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Failure("duplicate metadata key")
        result[key] = value
    return result
def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)
    except (OSError, UnicodeError, ValueError) as error:
        raise Failure("metadata is not valid JSON") from error
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def sha_bytes(data): return hashlib.sha256(data).hexdigest()

def safe_name(raw):
    need(isinstance(raw, str) and raw and "\x00" not in raw, "archive member name is invalid")
    need(not raw.startswith(("/", "\\")) and "\\" not in raw, "archive member is absolute or non-portable")
    value = raw.rstrip("/")
    need(value and all(part not in ("", ".", "..") for part in value.split("/")), "archive member traverses a path")
    return value
def derive(manifest, release, repository):
    need(isinstance(manifest, dict) and isinstance(release, dict), "metadata shape is invalid")
    need(manifest.get("name") == "codexy-devtools", "plugin manifest identity mismatch")
    version = manifest.get("version")
    need(isinstance(version, str) and SEMVER.fullmatch(version), "plugin manifest version is invalid")
    mrepo = manifest.get("repository")
    source = release.get("source")
    srepo = source.get("repository") if isinstance(source, dict) else None
    need(isinstance(mrepo, str) and isinstance(srepo, str), "metadata repository is missing")
    mrepo, srepo = mrepo.removesuffix(".git"), srepo.removesuffix(".git")
    need(mrepo == srepo == f"https://github.com/{repository}", "release repository is foreign or mismatched")
    artifact = release.get("artifact")
    need(isinstance(artifact, dict), "release artifact metadata is missing")
    tag = artifact.get("tag")
    need(tag == f"v{version}", "release tag is not bound to the manifest")
    url = artifact.get("url")
    need(isinstance(url, str), "release URL is missing")
    try:
        parsed = urlsplit(url)
    except ValueError as error:
        raise Failure("release URL is invalid") from error
    asset = parsed.path.rsplit("/", 1)[-1]
    expected_url = f"https://github.com/{repository}/releases/download/{tag}/{asset}"
    need(asset and "/" not in asset and parsed.scheme == "https" and parsed.netloc == "github.com", "release URL is unsafe")
    need(not parsed.query and not parsed.fragment and url == expected_url, "release URL binding mismatch")
    archive_sha = artifact.get("sha256")
    need(isinstance(archive_sha, str) and HEX64.fullmatch(archive_sha), "archive digest is invalid")
    need(isinstance(manifest.get("supportedPlatforms"), list) and PLATFORM in manifest["supportedPlatforms"], "Linux platform is unsupported")
    platforms = release.get("platforms")
    classes = release.get("classes")
    mirrored = classes.get("devtoolsMcp", {}).get("platforms") if isinstance(classes, dict) else None
    platforms = platforms.get(PLATFORM) if isinstance(platforms, dict) else None
    mirrored = mirrored.get(PLATFORM) if isinstance(mirrored, dict) else None
    need(isinstance(platforms, dict) and isinstance(mirrored, dict), "Linux runtime metadata is missing")
    members = []
    for server in ("codegraph", "lsp"):
        row = platforms.get(server)
        need(isinstance(row, dict) and row == mirrored.get(server), "runtime metadata mirror mismatch")
        name, checksum = row.get("path"), row.get("sha256")
        need(isinstance(name, str) and name == safe_name(name) and PurePosixPath(name).parent == PurePosixPath("runtime") and PurePosixPath(name).name == f"codexy-mcp-{server}-{PLATFORM}.bin", "Linux runtime member metadata is invalid")
        need(isinstance(checksum, str) and HEX64.fullmatch(checksum), "Linux binary digest is invalid")
        members.append({"server": server, "path": name, "sha256": checksum})
    return {"repository": mrepo, "plugin": manifest["name"], "version": version, "tag": tag, "url": url, "archive_sha256": archive_sha, "platform": PLATFORM, "members": members}
def metadata(root, repository):
    plugin = root / "plugins/codexy-devtools"
    return derive(load(plugin / ".codex-plugin/plugin.json"), load(plugin / "runtime-release.json"), repository)
def verify_archive(archive, expected):
    need(archive.is_file() and not archive.is_symlink() and archive.stat().st_size <= MAX_ARCHIVE, "runtime archive is missing or oversized"); need(sha(archive) == expected, "runtime archive digest mismatch")

def extract_verified(archive, output, expected):
    need(not os.path.lexists(output), "verified runtime directory is stale")
    output.mkdir(mode=0o700)
    selected, seen = {}, set()
    try:
        with tarfile.open(archive, "r:gz") as stream:
            for item in stream.getmembers():
                name = safe_name(item.name)
                need(name not in seen, "archive contains duplicate members")
                seen.add(name)
                need(not item.issym() and not item.islnk(), "archive contains a link")
                need(item.isdir() or item.isreg(), "archive contains a special member")
                if item.isreg():
                    need(not item.name.endswith("/") and item.size <= MAX_MEMBER, "archive file member is invalid")
                if name in expected:
                    need(item.isreg(), "required runtime member is not regular")
                    selected[name] = item
            need(set(selected) == set(expected), "required Linux runtime members are incomplete")
            records = []
            for name, checksum in expected.items():
                source = stream.extractfile(selected[name])
                need(source is not None, "required runtime member cannot be read")
                data = source.read(MAX_MEMBER + 1)
                need(len(data) == selected[name].size and len(data) <= MAX_MEMBER, "runtime member size is invalid")
                need(sha_bytes(data) == checksum and data.startswith(b"\x7fELF"), "runtime binary verification failed")
                target = output / PurePosixPath(name).name
                target.write_bytes(data)
                target.chmod(0o755)
                records.append({"path": name, "sha256": checksum, "bytes": len(data)})
    except (OSError, tarfile.TarError) as error:
        raise Failure("runtime archive cannot be inspected") from error
    names = sorted(PurePosixPath(name).name for name in expected)
    need(sorted(item.name for item in output.iterdir()) == names and all(item.is_file() and not item.is_symlink() for item in output.iterdir()), "verified runtime output is unsafe or extra")
    return records


CREDENTIAL = re.compile(r"(?i)(?<![A-Za-z0-9])(?:sk-[A-Za-z0-9]{8,}|gh[pousr]_[A-Za-z0-9]{8,}|github_pat_[A-Za-z0-9_]+|Bearer\s+\S+)|[A-Za-z][A-Za-z0-9+.-]*://[^/\s@]+(?:[:][^/\s@]*)?@")


def safe_receipt(value): need(not CREDENTIAL.search(json.dumps(value, sort_keys=True)), "preflight receipt contains credential material")


def hosted(root, receipt, output, repository):
    need(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted", "preflight is hosted-only")
    need(root.is_absolute() and root.is_dir() and not root.is_symlink(), "preflight source root is invalid")
    need(receipt.is_absolute() and receipt.parent.is_dir() and not os.path.lexists(receipt), "preflight receipt path is invalid")
    need(output.is_absolute() and output.parent.is_dir() and not os.path.lexists(output), "preflight runtime path is invalid")
    meta = metadata(root, repository)
    archive = receipt.parent / "runtime-release-asset.tar.gz"
    need(not os.path.lexists(archive), "preflight archive path is stale")
    try:
        subprocess.run(["curl", "--config", "/dev/null", "--fail", "--silent", "--show-error", "--location", "--proto", "=https", "--tlsv1.2", "--output", str(archive), meta["url"]], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        raise Failure("runtime release download failed") from error
    verify_archive(archive, meta["archive_sha256"])
    records = extract_verified(archive, output, {item["path"]: item["sha256"] for item in meta["members"]})
    result = {"schema": "codexy.a02.runtime-preflight.v1", "plugin": {"name": meta["plugin"], "version": meta["version"], "repository": meta["repository"]}, "release": {"tag": meta["tag"], "url": meta["url"], "archive_sha256": meta["archive_sha256"]}, "platform": meta["platform"], "members": records, "runtime_dir": str(output.resolve()), "download": {"method": "unauthenticated-https", "authenticated": False}, "decision": "PASS"}
    safe_receipt(result)
    receipt.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(output.resolve())


def make_archive(path, entries):
    with tarfile.open(path, "w:gz") as stream:
        for kind, name, data in entries:
            item = tarfile.TarInfo(name)
            if kind == "file":
                item.size = len(data)
                stream.addfile(item, io.BytesIO(data))
            elif kind == "dir":
                item.type = tarfile.DIRTYPE
                stream.addfile(item)
            elif kind in ("symlink", "hardlink"):
                item.type = tarfile.SYMTYPE if kind == "symlink" else tarfile.LNKTYPE
                item.linkname = "runtime/target"
                stream.addfile(item)
            else:
                item.type = tarfile.CHRTYPE
                stream.addfile(item)


def self_test():
    with tempfile.TemporaryDirectory() as temporary:
        root, repository = Path(temporary), "example/codexy"
        version = ".".join(["0"] * 3)
        paths = {server: f"runtime/codexy-mcp-{server}-{PLATFORM}.bin" for server in ("codegraph", "lsp")}
        blobs = {"codegraph": b"\x7fELFsynthetic-codegraph", "lsp": b"\x7fELFsynthetic-lsp"}
        expected = {paths[server]: sha_bytes(blobs[server]) for server in paths}
        manifest = {"name": "codexy-devtools", "version": version, "repository": f"https://github.com/{repository}", "supportedPlatforms": [PLATFORM]}
        release = {"artifact": {"tag": f"v{version}", "url": f"https://github.com/{repository}/releases/download/v{version}/runtime.tar.gz", "sha256": "0" * 64}, "classes": {"devtoolsMcp": {"platforms": {PLATFORM: {}}}}, "platforms": {PLATFORM: {}}, "source": {"repository": f"https://github.com/{repository}"}}
        for server, name in paths.items():
            row = {"path": name, "sha256": expected[name]}
            release["platforms"][PLATFORM][server] = row
            release["classes"]["devtoolsMcp"]["platforms"][PLATFORM][server] = row
        good = [("dir", "runtime/", None)] + [("file", paths[server], blobs[server]) for server in paths] + [("file", "docs/readme.txt", b"safe")]
        archive = root / "runtime.tar.gz"
        make_archive(archive, good)
        release["artifact"]["sha256"] = sha(archive)
        need(derive(manifest, release, repository)["version"] == version, "self-test metadata binding failed")
        extract_verified(archive, root / "verified", expected)
        cases = 0

        def reject(label, operation):
            nonlocal cases
            try:
                operation()
            except Failure:
                cases += 1
            else:
                raise Failure(f"self-test accepted {label}")

        for label, change in [
            ("foreign repository", lambda value: value["source"].update(repository="https://github.com/foreign/codexy")),
            ("stale tag", lambda value: value["artifact"].update(tag=value["artifact"]["tag"] + "x")),
            ("foreign URL", lambda value: value["artifact"].update(url="https://github.com/foreign/codexy/releases/download/" + value["artifact"]["tag"] + "/runtime.tar.gz")),
        ]:
            candidate = copy.deepcopy(release)
            change(candidate)
            reject(label, lambda candidate=candidate: derive(manifest, candidate, repository))
        reject("malformed metadata", lambda: derive({key: value for key, value in manifest.items() if key != "version"}, release, repository))
        duplicate = root / "duplicate.json"
        duplicate.write_text('{"name":1,"name":2}', encoding="utf-8")
        reject("duplicate metadata", lambda: load(duplicate))
        reject("archive digest", lambda: verify_archive(archive, "f" * 64))
        negatives = [
            ("traversal", good + [("file", "../escape", b"x")]),
            ("absolute member", good + [("file", "/escape", b"x")]),
            ("duplicate member", good + [("file", paths["codegraph"], blobs["codegraph"])]),
            ("symlink", good + [("symlink", "runtime/link", None)]),
            ("hard link", good + [("hardlink", "runtime/link", None)]),
            ("special member", good + [("special", "runtime/device", None)]),
            ("missing member", [("file", paths["codegraph"], blobs["codegraph"])]),
        ]
        for label, entries in negatives:
            candidate = root / (label.replace(" ", "-") + ".tar.gz")
            make_archive(candidate, entries)
            reject(label, lambda candidate=candidate: extract_verified(candidate, root / (candidate.stem + "-out"), expected))
        wrong = dict(expected)
        wrong[paths["lsp"]] = "0" * 64
        reject("binary digest", lambda: extract_verified(archive, root / "wrong-out", wrong))
        extra = root / "extra-out"
        extra.mkdir()
        (extra / "unexpected").write_bytes(b"x")
        reject("extra output", lambda: extract_verified(archive, extra, expected))
        need(cases == 15, f"self-test case inventory mismatch: {cases}")
    print("PREFLIGHT SELF-TEST PASS cases=15")


def main(arguments):
    try:
        if arguments == ["--self-test"]:
            self_test()
            return 0
        need(len(arguments) == 3, "usage: a02-runtime-preflight.sh ROOT RECEIPT RUNTIME_DIR")
        root, receipt, output = (Path(value) for value in arguments)
        hosted(root, receipt, output, os.environ.get("GITHUB_REPOSITORY", ""))
        return 0
    except Failure as error:
        print(f"A02 PREFLIGHT FAIL: {error}", file=sys.stderr)
        return 1


raise SystemExit(main(sys.argv[1:]))
PY
