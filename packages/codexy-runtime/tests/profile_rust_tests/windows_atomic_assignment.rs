use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn launcher() -> std::path::PathBuf {
    codexy_runtime::paths::repository_root().join("scripts/profile_rust_windows_launcher.py")
}

fn invoke(token: Option<u8>, marker: &Path) -> Result<Output, Box<dyn std::error::Error>> {
    let source = "import pathlib,sys; pathlib.Path(sys.argv[1]).write_text('spawned'); raise SystemExit(7)";
    let mut child = Command::new("python3")
        .args(["-I", "-S"])
        .arg(launcher())
        .args(["python3", "-c", source])
        .arg(marker)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(token) = token {
        child.stdin.as_mut().expect("control pipe").write_all(&[token])?;
    }
    drop(child.stdin.take());
    Ok(child.wait_with_output()?)
}

#[cfg(unix)]
#[test]
fn isolated_launcher_rejects_unreleased_tokens_without_spawning() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let eof = root.path().join("eof-marker");
    let wrong = root.path().join("wrong-marker");
    assert_eq!(invoke(None, &eof)?.status.code(), Some(64));
    assert_eq!(invoke(Some(b"x"[0]), &wrong)?.status.code(), Some(64));
    assert!(!eof.exists() && !wrong.exists());
    let released = root.path().join("released-marker");
    assert_eq!(invoke(Some(b"R"[0]), &released)?.status.code(), Some(7));
    assert_eq!(std::fs::read_to_string(released)?, "spawned");
    Ok(())
}

#[cfg(unix)]
#[test]
fn launcher_drains_after_release_failure() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
import importlib.util,pathlib,sys
spec=importlib.util.spec_from_file_location('launcher',sys.argv[1]); launcher=importlib.util.module_from_spec(spec); spec.loader.exec_module(launcher); events=[]
class Pipe:
 def write(self,_value): events.append('write'); raise OSError('release')
 def flush(self): events.append('flush')
 def close(self): events.append('control-close')
class Process:
 stdin=Pipe()
 def kill(self): events.append('kill')
 def wait(self): events.append('wait'); return 124
class Job:
 def assign(self,_process): events.append('assign')
 def terminate_and_wait(self): events.append('terminate')
 def close(self): events.append('job-close')
try: launcher.launch_windows_workload(Job(),pathlib.Path('.'),None,('cargo',),lambda *_args,**_kwargs: Process())
except OSError as error:
 if str(error)!='release': raise
else: raise SystemExit('release unexpectedly succeeded')
if events!=['assign','write','control-close','terminate','wait','job-close']: raise SystemExit(repr(events))
"#;
    let output = Command::new("python3")
        .args(["-I", "-S", "-c", source])
        .arg(launcher())
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[cfg(windows)]
#[test]
fn job_owns_immediate_spawn_before_root_returns() -> Result<(), Box<dyn std::error::Error>> {
    let profile = codexy_runtime::paths::repository_root().join("scripts/profile-rust-tests");
    let probe = r#"
import json,pathlib,runpy,shutil,subprocess,sys,tempfile,time,types
profile=pathlib.Path(sys.argv[1]); sys.path.insert(0,str(profile.parent)); module=runpy.run_path(profile)
work=pathlib.Path(tempfile.mkdtemp()); marker=work/'writer.pid'; paths=[]; real=subprocess
source="import pathlib,subprocess,sys;sys.stdout.buffer.write(b'first\\r\\n\\xce\\xbc-tail\\r\\n');sys.stdout.buffer.flush();p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'],stdout=sys.stdout.buffer,stderr=sys.stdout.buffer,close_fds=False);pathlib.Path(sys.argv[1]).write_text(str(p.pid));sys.stdin.buffer.read(1);raise SystemExit(7)"
class Root:
 def __init__(self,capture):
  self.child=real.Popen((sys.executable,'-c',source,str(marker)),stdin=real.PIPE,stdout=capture,stderr=real.STDOUT,close_fds=False); self.pid=self.child.pid; self._handle=self.child._handle
  deadline=time.monotonic()+2
  while not marker.exists() and time.monotonic()<deadline: time.sleep(.01)
  if not marker.exists(): raise RuntimeError('root did not spawn writer before Popen returned')
 def wait(self,timeout=None): return self.child.wait(timeout)
 def poll(self): return self.child.poll()
class Temp:
 def __init__(self,*_args,**_kwargs): self.path=pathlib.Path(tempfile.mkdtemp(dir=work)); paths.append(self.path)
 def __enter__(self): return str(self.path)
 def __exit__(self,*_args): shutil.rmtree(self.path)
legacy=Temp(); legacy_path=pathlib.Path(legacy.__enter__())
try:
 with (legacy_path/'cargo-output').open('wb',buffering=0) as capture:
  root=Root(capture); job=module['WindowsJob'](); job.assign(root)
  try: root.wait(.2)
  except real.TimeoutExpired: job.terminate_and_wait()
  finally: job.close()
 try: legacy.__exit__(None,None,None)
 except PermissionError: locked=True
 else: locked=False
finally:
 if marker.exists(): real.run(('taskkill','/F','/T','/PID',marker.read_text()),stdout=real.DEVNULL,stderr=real.DEVNULL)
 for path in paths: shutil.rmtree(path,ignore_errors=True)
if not locked: raise SystemExit('predecessor direct Popen did not leave the immediate writer outside the Job')
module['run_workload'].__globals__['WORKLOAD']=(sys.executable,'-c',source,str(marker))
marker.unlink()
result=module['run_workload'](work,1.0)
output,_elapsed,status,phases=result; pids=json.loads(phases['windows-job-pids-json']); images=json.loads(phases['windows-job-images-json'])
if output!='first\r\nμ-tail\r\n' or status!=7 or phases['windows-job-active-zero']!='drained' or phases['cargo-root-status']!='7' or not pids or not all(any(image.get('pid')==pid for image in images) for pid in pids): raise SystemExit(f'result={result!r}')
shutil.rmtree(work)
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(profile)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}

#[cfg(windows)]
#[test]
fn assignment_failure_reaps_unreleased_launcher() -> Result<(), Box<dyn std::error::Error>> {
    let launcher = launcher();
    let probe = r#"
import importlib.util,pathlib,shutil,subprocess,sys,tempfile,types
path=pathlib.Path(sys.argv[1]); sys.path.insert(0,str(path.parent)); spec=importlib.util.spec_from_file_location('launcher',path); launcher=importlib.util.module_from_spec(spec); spec.loader.exec_module(launcher)
root=pathlib.Path(tempfile.mkdtemp()); marker=root/'marker'; started=[]; closed=[]
native=__import__('profile_rust_windows_job').WindowsJob()
def spawn(*args,**kwargs):
 process=subprocess.Popen(*args,**kwargs); started.append(process); return process
class InvalidJob:
 def assign(self,_process): native.assign(types.SimpleNamespace(_handle=0))
 def close(self): closed.append(started[0].poll()); native.close()
with (root/'capture').open('wb',buffering=0) as capture:
 try: launcher.launch_windows_workload(InvalidJob(),root,capture,(sys.executable,'-c',"import pathlib,sys;pathlib.Path(sys.argv[1]).write_text('spawned')",str(marker)),spawn)
 except OSError as error: message=str(error)
 else: raise SystemExit('invalid assignment unexpectedly succeeded')
if 'AssignProcessToJobObject' not in message or marker.exists() or not started or started[0].poll() is None or closed!=[started[0].poll()]: raise SystemExit(f'message={message!r} marker={marker.exists()} started={started!r} closed={closed!r}')
shutil.rmtree(root)
"#;
    let output = Command::new("python")
        .args(["-c", probe])
        .arg(launcher)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
