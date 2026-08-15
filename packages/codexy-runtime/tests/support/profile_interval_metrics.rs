use std::ffi::OsStr;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static METRICS: OnceLock<Option<Mutex<IntervalMetrics>>> = OnceLock::new();
static EPOCH: OnceLock<Instant> = OnceLock::new();

struct IntervalMetrics {
    file: std::fs::File,
    owner_file: Option<std::fs::File>,
    session: String,
    producer: String,
    target: &'static str,
    sequence: u64,
}

pub(crate) struct CommandInterval {
    key: &'static str,
    family: &'static str,
    started: u128,
    caller: Option<&'static std::panic::Location<'static>>,
}

pub(crate) fn command_interval(key: &'static str, family: &'static str) -> Option<CommandInterval> {
    interval(key, family, None)
}

pub(crate) fn command_interval_at(
    key: &'static str,
    family: &'static str,
    caller: &'static std::panic::Location<'static>,
) -> Option<CommandInterval> {
    interval(key, family, Some(caller))
}

pub(crate) fn wrapper_interval(
    operation: &'static str,
    program: &OsStr,
) -> Option<CommandInterval> {
    let category = command_family(program);
    let key = match (operation, category) {
        ("output", "git") => "wrapper.output.git",
        ("output", "python") => "wrapper.output.python",
        ("output", "shell") => "wrapper.output.shell",
        ("output", "validator") => "wrapper.output.validator",
        ("output", _) => "wrapper.output.other",
        ("spawn", "git") => "wrapper.spawn.git",
        ("spawn", "python") => "wrapper.spawn.python",
        ("spawn", "shell") => "wrapper.spawn.shell",
        ("spawn", "validator") => "wrapper.spawn.validator",
        ("spawn", _) => "wrapper.spawn.other",
        _ => "wrapper.child-wait.other",
    };
    interval(key, category, None)
}

pub(crate) fn mcp_interval(key: &'static str) -> Option<CommandInterval> {
    interval(key, "other", None)
}

pub(crate) fn generic_interval(key: &'static str, family: &'static str) -> Option<CommandInterval> {
    interval(key, family, None)
}

fn interval(
    key: &'static str,
    family: &'static str,
    caller: Option<&'static std::panic::Location<'static>>,
) -> Option<CommandInterval> {
    METRICS.get_or_init(open_metrics).as_ref()?;
    let epoch = EPOCH.get_or_init(Instant::now);
    Some(CommandInterval {
        key,
        family,
        started: epoch.elapsed().as_nanos(),
        caller,
    })
}

impl Drop for CommandInterval {
    fn drop(&mut self) {
        let Some(file) = METRICS.get_or_init(open_metrics).as_ref() else {
            return;
        };
        let end = EPOCH.get_or_init(Instant::now).elapsed().as_nanos();
        if let Ok(mut metrics) = file.lock() {
            metrics.sequence += 1;
            let line = format!(
                "command-interval\tv2\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                metrics.session,
                metrics.target,
                metrics.producer,
                metrics.sequence,
                self.key,
                self.family,
                self.started,
                end
            );
            let _ = writeln!(metrics.file, "{line}");
            let owner_line = self.caller.and_then(|caller| {
                normalized_source_file(caller.file()).map(|source| {
                    format!(
                        "fixture-command-owner\tv1\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        metrics.session,
                        metrics.target,
                        metrics.producer,
                        metrics.sequence,
                        self.key,
                        self.family,
                        source,
                        caller.line(),
                        self.started,
                        end
                    )
                })
            });
            if let (Some(line), Some(owner_file)) = (owner_line, metrics.owner_file.as_mut()) {
                let _ = writeln!(owner_file, "{line}");
            }
        }
    }
}

fn open_metrics() -> Option<Mutex<IntervalMetrics>> {
    let directory = std::env::var_os("CODEXY_PROFILE_INTERVAL_METRICS_DIR")?;
    let session = std::env::var("CODEXY_PROFILE_INTERVAL_SESSION").ok()?;
    if session.len() != 32
        || !session
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return None;
    }
    std::fs::create_dir_all(&directory).ok()?;
    let pid = std::process::id();
    for slot in 1..=256 {
        let producer = format!("p{pid}-{slot}");
        if let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::Path::new(&directory).join(format!("interval-{producer}.metrics")))
        {
            let owner_file = std::env::var_os("CODEXY_PROFILE_INTERVAL_OWNER_METRICS_DIR")
                .and_then(|directory| {
                    std::fs::create_dir_all(&directory).ok()?;
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(
                            std::path::Path::new(&directory)
                                .join(format!("owner-interval-{producer}.metrics")),
                        )
                        .ok()
                });
            return Some(Mutex::new(IntervalMetrics {
                file,
                owner_file,
                session,
                producer,
                target: profiler_target(),
                sequence: 0,
            }));
        }
    }
    None
}

fn normalized_source_file(file: &str) -> Option<String> {
    let file = file.replace('\\', "/");
    let relative = file
        .strip_prefix("tests/")
        .or_else(|| file.rsplit_once("/tests/").map(|(_, suffix)| suffix))?;
    let mut parts: Vec<&str> = Vec::new();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| format!("tests/{}", parts.join("/")))
}

fn profiler_target() -> &'static str {
    let Some(name) = std::env::current_exe().ok().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
    }) else {
        return "other";
    };
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    let name = name
        .rsplit_once('-')
        .filter(|(_, hash)| hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map_or(name, |(base, _)| base);
    match name {
        "suite_all" => "suite_all",
        "suite_archive" => "suite_archive",
        _ => "other",
    }
}

pub(crate) fn command_family(program: &OsStr) -> &'static str {
    let name = std::path::Path::new(program)
        .file_name()
        .unwrap_or(program)
        .to_string_lossy()
        .to_ascii_lowercase();
    if matches!(name.as_str(), "git" | "git.exe") {
        "git"
    } else if matches!(
        name.as_str(),
        "python" | "python.exe" | "python3" | "python3.exe" | "py" | "py.exe"
    ) {
        "python"
    } else if matches!(
        name.as_str(),
        "sh" | "sh.exe" | "bash" | "bash.exe" | "cmd" | "cmd.exe" | "pwsh" | "pwsh.exe"
    ) {
        "shell"
    } else if name.starts_with("codexy-validate") || name == "validate-plugin-config.sh" {
        "validator"
    } else {
        "other"
    }
}
