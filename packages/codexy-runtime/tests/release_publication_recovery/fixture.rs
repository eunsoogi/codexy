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
import hashlib,json,os,pathlib,shutil,sys
root=pathlib.Path.cwd(); remote=root/'remote'; exists=root/'exists'; draft=root/'draft'; log=root/'log'; tag=os.environ['RELEASE_TAG']; commit=os.environ['ACTIVATION_COMMIT']
def assets():
 return [{'id':i+1,'name':p.name,'size':p.stat().st_size,'digest':'sha256:'+hashlib.sha256(p.read_bytes()).hexdigest()} for i,p in enumerate(sorted(remote.iterdir()))]
def state(api=False):
 s={'id':42,'name':tag,'tag_name':tag,'target_commitish':commit,'draft':draft.read_text().strip()=='true','prerelease':False,'assets':assets()}
 if api: s['immutable']=os.environ.get('FIXTURE_IMMUTABLE','true') == 'true'
 else: s={'id':s['id'],'name':s['name'],'tagName':s['tag_name'],'targetCommitish':s['target_commitish'],'isDraft':s['draft'],'isPrerelease':s['prerelease'],'assets':s['assets']}
 return s
args=sys.argv[1:]
if args[:2]==['release','view']:
 print('gh: Not Found (HTTP 404)',file=sys.stderr); sys.exit(1)
if args[:2]==['release','create']:
 if exists.exists(): print('release already exists',file=sys.stderr); sys.exit(1)
 exists.write_text('yes'); draft.write_text('true'); log.write_text(log.read_text()+'create\n' if log.exists() else 'create\n'); sys.exit()
if args[:2]==['release','download']:
 name=args[args.index('--pattern')+1]; directory=pathlib.Path(args[args.index('--dir')+1]); directory.mkdir(exist_ok=True); target=directory/name
 if target.exists(): sys.exit(1)
 shutil.copy(remote/name,target); sys.exit()
if args[:2]==['release','upload']:
 if draft.read_text().strip() != 'true': sys.exit(1)
 source=pathlib.Path(args[3]); shutil.copy(source,remote/source.name); log.write_text(log.read_text()+'upload '+source.name+'\n' if log.exists() else 'upload '+source.name+'\n'); sys.exit()
if args[:2]==['release','edit']:
 if draft.read_text().strip() != 'true': sys.exit(1)
 draft.write_text('false'); log.write_text(log.read_text()+'publish\n'); sys.exit()
if args and args[0]=='api':
 endpoint=args[1] if len(args)>1 else ''
 if endpoint.endswith('/releases?per_page=100'): print(json.dumps([state(True)] if exists.exists() else [])); sys.exit()
 if endpoint.endswith('/releases/42'): print(json.dumps(state(True))); sys.exit()
 print(json.dumps(state(True))); sys.exit()
sys.exit(1)
"# }
