use std::{
    error::Error,
    fs,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct LockedProbeIndex {
    _directory: tempfile::TempDir,
    source: PathBuf,
    source_bytes: Vec<u8>,
    authority: PathBuf,
    porcelain: Vec<u8>,
    private: PrivateIndex,
}

impl LockedProbeIndex {
    pub(super) fn new(repository: &Path) -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        let source = source_index_path(repository, directory.path())?;
        let source_bytes = fs::read(&source)?;
        let authority = directory.path().join("authority-index");
        fs::copy(&source, &authority)?;
        let authority_tree = git_text(repository, &authority, ["write-tree"])?;
        let porcelain = git_output(repository, &authority, ["status", "--porcelain"])?;
        let private = PrivateIndex::from_source(repository, &source, &authority_tree)?;
        Ok(Self {
            _directory: directory,
            source,
            source_bytes,
            authority,
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

    fn probe(&self, repository: &Path) -> Result<(), Box<dyn Error>> {
        assert_eq!(git_text(repository, self.private.path(), ["write-tree"])?, self.tree());
        assert_eq!(
            git_output(repository, self.private.path(), ["status", "--porcelain"] )?,
            self.porcelain
        );
        Ok(())
    }

    pub(super) fn assert_unchanged(&self, repository: &Path) -> Result<(), Box<dyn Error>> {
        self.probe(repository)?;
        assert_eq!(fs::read(&self.source)?, self.source_bytes);
        assert_eq!(
            git_output(repository, &self.authority, ["status", "--porcelain"] )?,
            self.porcelain
        );
        assert_eq!(git_text(repository, &self.authority, ["write-tree"])?, self.tree());
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
        let tree = git_text(repository, &path, ["write-tree"])
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

fn source_index_path(repository: &Path, directory: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let bootstrap = directory.join("bootstrap-index");
    let git_dir = PathBuf::from(git_text(repository, &bootstrap, ["rev-parse", "--git-dir"])?);
    let git_dir = if git_dir.is_absolute() {
        git_dir
    } else {
        repository.join(git_dir)
    };
    Ok(git_dir.join("index"))
}

fn git_output<const N: usize>(
    repository: &Path,
    index: &Path,
    arguments: [&str; N],
) -> Result<Vec<u8>, Box<dyn Error>> {
    let output = Command::new("git")
        .args(arguments)
        .env("GIT_INDEX_FILE", index)
        .current_dir(repository)
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(invalid_index("Git command failed for private index"))
    }
}

fn git_text<const N: usize>(
    repository: &Path,
    index: &Path,
    arguments: [&str; N],
) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(git_output(repository, index, arguments)?)?
        .trim()
        .to_owned())
}

fn invalid_index(message: &str) -> Box<dyn Error> {
    Box::new(IoError::new(ErrorKind::InvalidData, message))
}

struct TemporaryRepository {
    directory: tempfile::TempDir,
}

impl TemporaryRepository {
    fn new() -> Result<Self, Box<dyn Error>> {
        let directory = tempfile::tempdir()?;
        for arguments in [
            ["init"].as_slice(),
            ["config", "user.email", "test@example.invalid"].as_slice(),
            ["config", "user.name", "Test"].as_slice(),
        ] {
            run_git(directory.path(), arguments)?;
        }
        fs::write(directory.path().join("control.txt"), "before\n")?;
        run_git(directory.path(), &["add", "control.txt"])?;
        run_git(directory.path(), &["commit", "-m", "before"])?;
        fs::write(directory.path().join("control.txt"), "after\n")?;
        run_git(directory.path(), &["add", "control.txt"])?;
        run_git(directory.path(), &["commit", "-m", "after"])?;
        Ok(Self { directory })
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }
}

fn run_git(repository: &Path, arguments: &[&str]) -> Result<(), Box<dyn Error>> {
    if Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .status()?
        .success()
    {
        Ok(())
    } else {
        Err(invalid_index("temporary Git fixture setup failed"))
    }
}

#[test]
fn locked_source_index_initializes_and_probes_only_through_private_indexes(
) -> Result<(), Box<dyn Error>> {
    let fixture = TemporaryRepository::new()?;
    let directory = tempfile::tempdir()?;
    let source = source_index_path(fixture.path(), directory.path())?;
    let source_bytes = fs::read(&source)?;
    fs::write(source.with_extension("lock"), b"held")?;
    let index = LockedProbeIndex::new(fixture.path())?;
    index.probe(fixture.path())?;
    index.assert_unchanged(fixture.path())?;
    assert_eq!(fs::read(source)?, source_bytes);
    Ok(())
}

#[test]
fn private_index_rejects_missing_invalid_and_wrong_authority_sources(
) -> Result<(), Box<dyn Error>> {
    let fixture = TemporaryRepository::new()?;
    let directory = tempfile::tempdir()?;
    let source = source_index_path(fixture.path(), directory.path())?;
    let authority = tempfile::tempdir()?;
    let authority_index = authority.path().join("authority-index");
    fs::copy(&source, &authority_index)?;
    let authority_tree = git_text(fixture.path(), &authority_index, ["write-tree"])?;
    let missing = directory.path().join("missing-index");
    let missing = expect_error(PrivateIndex::from_source(fixture.path(), &missing, &authority_tree));
    assert_eq!(missing.to_string(), "inherited Git index must be an existing file");

    let invalid = directory.path().join("invalid-index");
    fs::write(&invalid, b"not a Git index")?;
    let invalid = expect_error(PrivateIndex::from_source(fixture.path(), &invalid, &authority_tree));
    assert_eq!(invalid.to_string(), "private Git index cannot resolve repository tree");

    let wrong = directory.path().join("wrong-index");
    git_output(fixture.path(), &wrong, ["read-tree", "HEAD^"])?;
    let wrong = expect_error(PrivateIndex::from_source(fixture.path(), &wrong, &authority_tree));
    assert_eq!(wrong.to_string(), "private Git index tree differs from authoritative tree");
    Ok(())
}

fn expect_error<T>(result: Result<T, Box<dyn Error>>) -> Box<dyn Error> {
    match result {
        Ok(_) => panic!("invalid inherited index was accepted"),
        Err(error) => error,
    }
}
