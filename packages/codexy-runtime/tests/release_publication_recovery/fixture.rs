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
root=pathlib.Path.cwd(); remote=root/'remote'; exists=root/'exists'; created=root/'created'; visibility=root/'visibility'; draft=root/'draft'; log=root/'log'; reads=root/'reads'; patch_mode=root/'patch-mode'; tag=os.environ['RELEASE_TAG']; commit=os.environ['ACTIVATION_COMMIT']
def append_log(value):
 log.write_text(log.read_text()+value+'\n' if log.exists() else value+'\n')
def append_read(value):
 reads.write_text(reads.read_text()+value+'\n' if reads.exists() else value+'\n')
def assets():
 return [{'id':i+1,'name':p.name,'size':p.stat().st_size,'digest':'sha256:'+hashlib.sha256(p.read_bytes()).hexdigest()} for i,p in enumerate(sorted(remote.iterdir()))]
def state(api=False):
 s={'id':42,'name':tag,'tag_name':tag,'target_commitish':commit,'draft':draft.read_text().strip()=='true','prerelease':False,'upload_url':'https://uploads.github.com/repos/eunsoogi/codexy/releases/42/assets{?name,label}','assets':assets()}
 if api: s['immutable']=os.environ.get('FIXTURE_IMMUTABLE','true') == 'true'
 else: s={'id':s['id'],'name':s['name'],'tagName':s['tag_name'],'targetCommitish':s['target_commitish'],'isDraft':s['draft'],'isPrerelease':s['prerelease'],'assets':s['assets']}
 return s
args=sys.argv[1:]
def endpoint():
 for value in args[1:]:
  if value.startswith('repos/') or value.startswith('https://'): return value
 return ''
def method():
 return args[args.index('--method')+1] if '--method' in args else 'GET'
def input_path():
 return args[args.index('--input')+1] if '--input' in args else ''
def has_field(flag,value):
 for index,arg in enumerate(args[:-1]):
  if arg == flag and args[index+1] == value: return True
 return False
def complete_patch():
 return all(has_field(flag,value) for flag,value in [('-f',f'tag_name={tag}'),('-f',f'target_commitish={commit}'),('-f',f'name={tag}'),('-F','draft=false'),('-F','prerelease=false')])
if args[:2]==['release','view']:
 if created.exists(): visibility.write_text('visible'); print(json.dumps(state(False))); sys.exit()
 print('gh: Not Found (HTTP 404)',file=sys.stderr); sys.exit(1)
if args[:2]==['release','create']:
 if exists.exists(): print('release already exists',file=sys.stderr); sys.exit(1)
 exists.write_text('yes'); draft.write_text('true'); visibility.write_text('hidden'); append_log('legacy-create'); print('https://github.com/eunsoogi/codexy/releases/tag/untagged-create-race'); sys.exit()
if args[:2] in (['release','download'],['release','upload'],['release','edit']):
 print('gh: Not Found (HTTP 404) tag-selected draft route',file=sys.stderr); sys.exit(1)
if args and args[0]=='api':
 url=endpoint()
 if method() == 'POST' and url.endswith('/releases'):
  if exists.exists(): print('release already exists',file=sys.stderr); sys.exit(1)
  exists.write_text('yes'); draft.write_text('true'); created.write_text('yes'); visibility.write_text('hidden'); append_log('api-create'); print(json.dumps(state(True))); sys.exit()
 if url.endswith('/releases?per_page=100'):
  if visibility.exists() and visibility.read_text() == 'hidden':
   visibility.write_text('visible'); print('[]'); sys.exit()
  print(json.dumps([state(True)] if exists.exists() else [])); sys.exit()
 if method() == 'POST' and '/releases/42/assets?name=' in url:
  if not url.startswith('https://uploads.github.com/'): print('release asset upload used the API host',file=sys.stderr); sys.exit(1)
  name=urllib.parse.unquote(url.split('?name=',1)[1]); shutil.copy(input_path(),remote/name); append_log('api-upload '+name); print(json.dumps({'name':name})); sys.exit()
 if '/releases/assets/' in url:
  asset_id=int(url.rsplit('/',1)[1]); path=sorted(remote.iterdir())[asset_id-1]; append_read('api-download '+path.name); sys.stdout.buffer.write(path.read_bytes()); sys.exit()
 if method() == 'PATCH' and url.endswith('/releases/42'):
  complete=complete_patch()
  if complete: append_log('patch-complete')
  forced=patch_mode.exists() and patch_mode.read_text().strip() == '422'
  if not complete or forced:
   append_log('patch-rejected')
   print('HTTP/2.0 422 Unprocessable Entity',file=sys.stderr)
   print('Authorization: Bearer fixture-token',file=sys.stderr)
   print('Content-Type: application/json',file=sys.stderr)
   print('',file=sys.stderr)
   print('{"message":"Validation Failed","token":"fixture-token"}',file=sys.stderr)
   sys.exit(1)
  draft.write_text('false'); append_log('publish'); print(json.dumps(state(True))); sys.exit()
 if url.endswith('/releases/42'): print(json.dumps(state(True))); sys.exit()
 if '/releases/tags/' in url:
  print('gh: Not Found (HTTP 404) tag endpoint',file=sys.stderr); sys.exit(1)
 print('unexpected release API endpoint',url,file=sys.stderr); sys.exit(1)
sys.exit(1)
"# }

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::{Fixture, ASSETS};

    #[test]
    fn finalizer_reports_a_bounded_422_for_a_complete_public_update()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = Fixture::new(&ASSETS, false, false)?;
        let publish = fixture.run_with_policy("publish-verified-release", false, true)?;
        assert!(publish.status.success(), "{}", String::from_utf8_lossy(&publish.stderr));
        fs::write(fixture.root.join("patch-mode"), "422\n")?;
        let finalize = fixture.run_with_policy(
            "finalize-verified-release",
            fixture.last_baseline_created()?,
            true,
        )?;
        assert!(!finalize.status.success());
        let stderr = String::from_utf8_lossy(&finalize.stderr);
        let log = fixture.read("log")?;
        assert!(log.contains("patch-complete"), "{log}");
        assert!(log.contains("patch-rejected"), "{log}");
        assert!(stderr.contains("finalize-release status=422"), "{stderr}");
        assert!(stderr.contains("Validation Failed"), "{stderr}");
        assert!(!stderr.contains("fixture-token"), "{stderr}");
        assert!(stderr.len() <= 1_024, "diagnostic was not bounded: {}", stderr.len());
        assert!(!fixture.root.join("final-release-state.json").exists());
        assert!(!log.contains("publish\n"), "{log}");
        Ok(())
    }
}
