use std::{
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct LockedProbeIndex {
    directory: tempfile::TempDir,
    inherited: PathBuf,
    inherited_bytes: Vec<u8>,
    porcelain: Vec<u8>,
    private: PrivateIndex,
}

impl LockedProbeIndex {
    pub(super) fn new(repository: &Path) -> Result<Self, Box<dyn Error>> {
        let inherited = git_index_path(repository)?;
        let inherited_bytes = fs::read(&inherited)?;
        let porcelain = git_output(repository, None, ["status", "--porcelain"])?;
        let authority_tree = git_text(repository, None, ["write-tree"])?;
        let directory = tempfile::tempdir()?;
        let shared = directory.path().join("inherited-index");
        fs::copy(&inherited, &shared)?;
        fs::write(shared.with_extension("lock"), b"held")?;
        let private = PrivateIndex::from_source(repository, &shared, &authority_tree)?;
        Ok(Self {
            directory,
            inherited,
            inherited_bytes,
            porcelain,
            private,
        })
    }

    pub(super) fn path(&self) -> &Path {
        self.private.path()
    }

    pub(super) fn tree(&self) -> &str {
        self.private.tree()
    }

    pub(super) fn assert_unchanged(&self, repository: &Path) -> Result<(), Box<dyn Error>> {
        assert_eq!(fs::read(&self.inherited)?, self.inherited_bytes);
        assert_eq!(
            git_output(repository, None, ["status", "--porcelain"] )?,
            self.porcelain
        );
        assert!(self.directory.path().join("inherited-index.lock").is_file());
        Ok(())
    }
}

struct PrivateIndex {
    _directory: tempfile::TempDir,
    path: PathBuf,
    tree: String,
}

impl PrivateIndex {
    fn from_source(
        repository: &Path,
        source: &Path,
        authority_tree: &str,
    ) -> Result<Self, Box<dyn Error>> {
        if !source.is_file() {
            return Err(invalid_index("inherited Git index must be an existing file"));
        }
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("private-index");
        fs::copy(source, &path)?;
        let tree = git_text(repository, Some(&path), ["write-tree"])
            .map_err(|_| invalid_index("private Git index cannot resolve repository tree"))?;
        if tree != authority_tree {
            return Err(invalid_index("private Git index tree differs from authoritative tree"));
        }
        Ok(Self {
            _directory: directory,
            path,
            tree,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn tree(&self) -> &str {
        &self.tree
    }
}

fn git_index_path(repository: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = git_text(repository, None, ["rev-parse", "--git-path", "index"])?;
    let path = PathBuf::from(path);
    Ok(if path.is_absolute() { path } else { repository.join(path) })
}

fn git_output<const N: usize>(
    repository: &Path,
    index: Option<&Path>,
    arguments: [&str; N],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut command = Command::new("git");
    command.args(arguments).current_dir(repository);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command.output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(invalid_index("Git command failed for private index"))
    }
}

fn git_text<const N: usize>(
    repository: &Path,
    index: Option<&Path>,
    arguments: [&str; N],
) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(git_output(repository, index, arguments)?)?
        .trim()
        .to_owned())
}

fn invalid_index(message: &str) -> Box<dyn Error> {
    Box::new(IoError::new(ErrorKind::InvalidData, message))
}

#[test]
fn private_index_rejects_missing_and_invalid_sources_without_fallback(
) -> Result<(), Box<dyn Error>> {
    let repository = codexy_runtime::paths::repository_root();
    let authority_tree = git_text(&repository, None, ["write-tree"])?;
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("missing-index");
    let missing = match PrivateIndex::from_source(&repository, &missing, &authority_tree) {
        Ok(_) => panic!("missing inherited index was accepted"),
        Err(error) => error,
    };
    assert_eq!(missing.to_string(), "inherited Git index must be an existing file");

    let invalid = directory.path().join("invalid-index");
    fs::write(&invalid, b"not a Git index")?;
    let invalid = match PrivateIndex::from_source(&repository, &invalid, &authority_tree) {
        Ok(_) => panic!("invalid inherited index was accepted"),
        Err(error) => error,
    };
    assert_eq!(
        invalid.to_string(),
        "private Git index cannot resolve repository tree"
    );
    Ok(())
}
