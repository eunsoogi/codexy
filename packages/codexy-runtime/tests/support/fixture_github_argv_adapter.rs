use std::path::{Path, PathBuf};

pub(crate) const GITHUB_ARGV_ADAPTER: &str = r##"#!/usr/bin/env python3
import os
import pathlib
import subprocess
import sys

def fail(message):
    print(f'fixture gh argv transport: {message}', file=sys.stderr)
    sys.exit(2)

def read_transport():
    fields = sys.stdin.buffer.read().split(b'\0')
    if fields[-1:] != [b''] or len(fields) < 4:
        fail('missing typed launch transport')
    try:
        repository, payload, launcher, *arguments = (field.decode('utf-8') for field in fields[:-1])
    except UnicodeDecodeError as error:
        fail(f'invalid typed launch transport: {error}')
    if not repository or not payload or not launcher:
        fail('missing typed launch transport')
    return repository, payload, launcher, arguments

def fixture_native_windows():
    return os.name == 'nt' or os.environ.get('CODEXY_FIXTURE_FORCE_NATIVE_WINDOWS') == '1'

def projected_path(option, value):
    if not fixture_native_windows():
        return value
    converter = os.environ.get('FIXTURE_GH_CYGPATH')
    if not converter:
        fail('missing native filesystem converter')
    try:
        return subprocess.check_output(
            [converter, option, '--', value], text=True, stderr=subprocess.PIPE
        ).rstrip('\r\n')
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f'filesystem conversion: {error}')

def native_path(value):
    return projected_path('-w', value)

def posix_payload_path(value):
    return projected_path('-u', value)

def native_arguments(args):
    file_indices = set()
    if args[:2] == ['release', 'download']:
        file_indices.update(index + 1 for index, value in enumerate(args) if value == '--dir')
    elif args[:2] == ['release', 'upload'] and len(args) > 3:
        file_indices.add(3)
    elif args[:2] == ['attestation', 'verify'] and len(args) > 2:
        file_indices.add(2)
    return [native_path(value) if index in file_indices else value for index, value in enumerate(args)]

def payload_is_posix(path):
    return pathlib.Path(path).read_bytes().startswith(b'#!/bin/sh')

repository, payload, launcher, arguments = read_transport()
os.environ['GITHUB_REPOSITORY'] = repository
os.environ['CODEXY_FIXTURE_GH_TRANSPORT'] = '1'
if payload_is_posix(payload):
    payload = posix_payload_path(payload)
else:
    arguments = native_arguments(arguments)
try:
    result = subprocess.run([launcher, payload, *arguments], check=False)
except OSError as error:
    fail(f'native launch: {error}')
sys.exit(result.returncode)
"##;

pub(crate) fn fixture_github_argv_adapter_path(path: &Path) -> PathBuf {
    path.parent()
        .expect("fixture script parent")
        .join(".codexy-fixture-github-argv.py")
}

/// Returns the executable that projects fixture filesystem paths at the native
/// GitHub-mock boundary. On Windows this is the host `cygpath.exe`; elsewhere
/// a fixture may provide its own POSIX test double at `bin/cygpath`.
pub(crate) fn fixture_github_cygpath_path(root: &Path) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        crate::support::executable_path("cygpath")
    }
    #[cfg(not(windows))]
    {
        Ok(root.join("bin/cygpath"))
    }
}
