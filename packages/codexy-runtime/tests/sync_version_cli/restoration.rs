use std::{
    fs,
    path::{Path, PathBuf},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) const VERSION_FIXTURE_PATHS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".agents/plugins/release-publish-contract.json",
    ".agents/plugins/runtime-activation.json",
    "packages/codexy-runtime/Cargo.lock",
    "packages/codexy-runtime/Cargo.toml",
    "packages/codexy-runtime/src/version/bootstrap.rs",
    "packages/getcodexy/pyproject.toml",
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
    "packages/getcodexy/uv.lock",
    "plugins/codexy/.codex-plugin/plugin.json",
    "plugins/codexy-devtools/.codex-plugin/plugin.json",
    "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
    "plugins/codexy-github/.codex-plugin/plugin.json",
    "README.md",
    "README.ko.md",
];

pub(super) struct ByteSnapshot(Vec<(PathBuf, Vec<u8>)>);

impl ByteSnapshot {
    pub(super) fn capture(root: &Path, paths: &[&str]) -> TestResult<Self> {
        paths
            .iter()
            .map(|relative| {
                let path = root.join(relative);
                Ok((path.clone(), fs::read(path)?))
            })
            .collect::<TestResult<Vec<_>>>()
            .map(Self)
    }

    pub(super) fn guard(&self) -> Restoration<'_> {
        Restoration {
            snapshot: self,
            restored: false,
        }
    }

    fn restore(&self) -> TestResult {
        for (path, bytes) in &self.0 {
            fs::write(path, bytes)?;
        }
        for (path, bytes) in &self.0 {
            assert_eq!(
                fs::read(path)?,
                *bytes,
                "fixture bytes were not restored: {}",
                path.display()
            );
        }
        Ok(())
    }
}

pub(super) struct Restoration<'a> {
    snapshot: &'a ByteSnapshot,
    restored: bool,
}

impl Restoration<'_> {
    pub(super) fn restore_checked(&mut self) -> TestResult {
        self.snapshot.restore()?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for Restoration<'_> {
    fn drop(&mut self) {
        if !self.restored {
            if let Err(error) = self.snapshot.restore() {
                if std::thread::panicking() {
                    eprintln!("fixture restoration failed during unwinding: {error}");
                } else {
                    panic!("fixture restoration failed: {error}");
                }
            }
        }
    }
}
