pub(crate) fn gh_fixture() -> &'static str {
    r#"#!/usr/bin/env python3
import hashlib
import json
import os
import pathlib
import shutil
import sys

root = pathlib.Path.cwd()
remote = root / 'remote'
exists = root / 'exists'
draft = root / 'draft'
log = root / 'log'
tag = 'v9.9.9'
commit = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
args = sys.argv[1:]

def fail(message):
    print(f'fixture gh contract: {message}; argv={args!r}; cwd={root}; repository={os.environ.get("GITHUB_REPOSITORY")!r}', file=sys.stderr)
    sys.exit(2)

def repository():
    repo = os.environ.get('GITHUB_REPOSITORY')
    if repo != 'eunsoogi/codexy':
        fail('logical repository environment')
    return repo

def route():
    repo = repository()
    if args[0] == 'api' and not any(value.startswith(f'repos/{repo}/') for value in args):
        fail('logical API route')
    if '--repo' in args and args[args.index('--repo') + 1] != repo:
        fail('logical release repository')

def fixture_path(value):
    path = pathlib.Path(value)
    if path.is_absolute():
        return path
    if os.name == 'nt' and len(value) > 3 and value[0] == '/' and value[2] == '/':
        return pathlib.Path(value[1].upper() + ':/' + value[3:])
    return path

def download_directory():
    directory = fixture_path(args[args.index('--dir') + 1])
    if not directory.is_absolute():
        fail('native release download directory')
    return directory

def assets():
    return [
        {'id': i + 1, 'name': path.name, 'size': path.stat().st_size, 'digest': 'sha256:' + hashlib.sha256(path.read_bytes()).hexdigest()}
        for i, path in enumerate(sorted(remote.iterdir()))
    ]

def state(api=False):
    result = {
        'id': 42,
        'name': tag,
        'tag_name': tag,
        'target_commitish': commit,
        'draft': draft.read_text().strip() == 'true',
        'prerelease': False,
        'assets': assets(),
    }
    if api:
        result['immutable'] = os.environ.get('FIXTURE_IMMUTABLE', 'true') == 'true'
        return result
    return {
        'id': result['id'],
        'name': result['name'],
        'tagName': result['tag_name'],
        'targetCommitish': result['target_commitish'],
        'isDraft': result['draft'],
        'isPrerelease': result['prerelease'],
        'assets': result['assets'],
    }

route()
if args[:2] == ['release', 'view']:
    if not exists.exists():
        sys.exit(1)
    graph = state()
    graph['id'] = 'node-42'
    print(json.dumps(graph))
    sys.exit()
if args[:2] == ['release', 'create']:
    exists.write_text('yes')
    draft.write_text('true')
    log.write_text(log.read_text() + 'create\n' if log.exists() else 'create\n')
    sys.exit()
if args[:2] == ['release', 'download']:
    name = args[args.index('--pattern') + 1]
    directory = download_directory()
    directory.mkdir(exist_ok=True)
    target = directory / name
    if target.exists():
        sys.exit(1)
    shutil.copy(remote / name, target)
    sys.exit()
if args[:2] == ['release', 'upload']:
    if draft.read_text().strip() != 'true':
        sys.exit(1)
    source = fixture_path(args[3])
    shutil.copy(source, remote / source.name)
    log.write_text(log.read_text() + 'upload ' + source.name + '\n' if log.exists() else 'upload ' + source.name + '\n')
    sys.exit()
if args[:2] == ['release', 'edit']:
    if draft.read_text().strip() != 'true':
        sys.exit(1)
    draft.write_text('false')
    log.write_text(log.read_text() + 'publish\n')
    sys.exit()
if args and args[0] == 'api':
    print(json.dumps(state(True)))
    sys.exit()
fail('unsupported invocation')
"#
}
