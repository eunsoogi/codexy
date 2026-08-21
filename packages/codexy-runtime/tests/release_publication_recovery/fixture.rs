use std::path::Path;

pub(crate) fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn git_fixture() -> &'static str { r#"#!/bin/sh
case "$1" in fetch|merge-base) exit 0 ;; rev-parse) printf '%s\n' "$ACTIVATION_COMMIT" ;; ls-remote) printf '%s\trefs/tags/%s\n' "$ACTIVATION_COMMIT" "$RELEASE_TAG" ;; *) exit 1 ;; esac
"# }

pub(crate) fn gh_fixture() -> &'static str { r#"#!/usr/bin/env python3
import hashlib,json,os,pathlib,shutil,sys,urllib.parse
root=pathlib.Path.cwd(); remote=root/'remote'; exists=root/'exists'; draft=root/'draft'; log=root/'log'; reads=root/'reads'; tag=os.environ['RELEASE_TAG']; commit=os.environ['ACTIVATION_COMMIT']
def append_log(value):
 log.write_text(log.read_text()+value+'\n' if log.exists() else value+'\n')
def append_read(value):
 reads.write_text(reads.read_text()+value+'\n' if reads.exists() else value+'\n')
def assets():
 return [{'id':i+1,'name':p.name,'size':p.stat().st_size,'digest':'sha256:'+hashlib.sha256(p.read_bytes()).hexdigest()} for i,p in enumerate(sorted(remote.iterdir()))]
def state(api=False):
 s={'id':42,'name':tag,'tag_name':tag,'target_commitish':commit,'draft':draft.read_text().strip()=='true','prerelease':False,'assets':assets()}
 if api: s['immutable']=os.environ.get('FIXTURE_IMMUTABLE','true') == 'true'
 else: s={'id':s['id'],'name':s['name'],'tagName':s['tag_name'],'targetCommitish':s['target_commitish'],'isDraft':s['draft'],'isPrerelease':s['prerelease'],'assets':s['assets']}
 return s
args=sys.argv[1:]
def endpoint():
 for value in args[1:]:
  if value.startswith('repos/'): return value
 return ''
def method():
 return args[args.index('--method')+1] if '--method' in args else 'GET'
def input_path():
 return args[args.index('--input')+1] if '--input' in args else ''
if args[:2]==['release','view']:
 print('gh: Not Found (HTTP 404)',file=sys.stderr); sys.exit(1)
if args[:2]==['release','create']:
 if exists.exists(): print('release already exists',file=sys.stderr); sys.exit(1)
 exists.write_text('yes'); draft.write_text('true'); append_log('create'); sys.exit()
if args[:2] in (['release','download'],['release','upload'],['release','edit']):
 print('gh: Not Found (HTTP 404) tag-selected draft route',file=sys.stderr); sys.exit(1)
if args and args[0]=='api':
 url=endpoint()
 if url.endswith('/releases?per_page=100'): print(json.dumps([state(True)] if exists.exists() else [])); sys.exit()
 if method() == 'POST' and '/releases/42/assets?name=' in url:
  name=urllib.parse.unquote(url.split('?name=',1)[1]); shutil.copy(input_path(),remote/name); append_log('api-upload '+name); print(json.dumps({'name':name})); sys.exit()
 if '/releases/assets/' in url:
  asset_id=int(url.rsplit('/',1)[1]); path=sorted(remote.iterdir())[asset_id-1]; append_read('api-download '+path.name); sys.stdout.buffer.write(path.read_bytes()); sys.exit()
 if method() == 'PATCH' and url.endswith('/releases/42'):
  draft.write_text('false'); append_log('publish'); print(json.dumps(state(True))); sys.exit()
 if url.endswith('/releases/42'): print(json.dumps(state(True))); sys.exit()
 if '/releases/tags/' in url:
  print('gh: Not Found (HTTP 404) tag endpoint',file=sys.stderr); sys.exit(1)
 print('unexpected release API endpoint',url,file=sys.stderr); sys.exit(1)
sys.exit(1)
"# }
