use std::{fs, path::Path, process::Command};

use tempfile::tempdir;

const FIRST_DECLARATION: &str = "bundled_platforms=\"darwin-arm64 linux-x86_64\"\n";
const ACTIVATED_DECLARATION: &str = "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"\n";

#[test]
fn candidate_assembly_accepts_first_and_subsequent_truthful_wrapper_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    for declaration in [FIRST_DECLARATION, ACTIVATED_DECLARATION] {
        let fixture = CandidateFixture::new(declaration)?;
        let output = fixture.assemble();
        assert!(
            output.status.success(),
            "candidate assembly failed for {declaration:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        for server in ["lsp", "codegraph"] {
            let wrapper = fs::read_to_string(
                fixture
                    .root()
                    .join("dist/candidate/plugins/codexy/mcp")
                    .join(format!("codexy-mcp-{server}")),
            )?;
            assert_eq!(wrapper.replace("\r\n", "\n"), ACTIVATED_DECLARATION);
        }
    }
    Ok(())
}

#[test]
fn candidate_assembly_rejects_nonexact_wrapper_platform_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    for declaration in [
        "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64 windows-x86_64\"\n".into(),
        "bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n".into(),
        format!("{FIRST_DECLARATION}bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        "not_bundled_platforms=\"darwin-arm64 linux-x86_64\"\n".into(),
        "# bundled_platforms=\"darwin-arm64 linux-x86_64\"\n".into(),
        format!("{FIRST_DECLARATION}{FIRST_DECLARATION}"),
        format!("{FIRST_DECLARATION}{ACTIVATED_DECLARATION}"),
        "#!/bin/sh\necho wrapper\n".into(),
    ] {
        let fixture = CandidateFixture::new(&declaration)?;
        let output = fixture.assemble();
        assert!(
            !output.status.success(),
            "candidate assembly accepted malformed declaration {declaration:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("candidate wrapper platform declaration mismatch")
        );
    }
    Ok(())
}

struct CandidateFixture {
    temp: tempfile::TempDir,
    source_commit: String,
}

impl CandidateFixture {
    fn new(wrapper_declaration: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();
        let plugin = root.join("plugins/codexy");
        fs::create_dir_all(plugin.join(".codex-plugin"))?;
        fs::create_dir_all(plugin.join("mcp"))?;
        fs::create_dir_all(root.join("staged-runtime"))?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(root.join("test-bin"))?;
        fs::write(plugin.join(".codex-plugin/plugin.json"), "{}\n")?;
        for server in ["lsp", "codegraph"] {
            fs::write(plugin.join("mcp").join(format!("codexy-mcp-{server}")), wrapper_declaration)?;
            for (platform, extension) in [
                ("darwin-arm64", "bin"),
                ("linux-x86_64", "bin"),
                ("windows-x86_64", "exe"),
            ] {
                fs::write(
                    root.join("staged-runtime").join(format!("codexy-mcp-{server}-{platform}.{extension}")),
                    format!("{server}-{platform}\n"),
                )?;
            }
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/assemble-runtime-candidate");
        fs::copy(source, root.join("scripts/assemble-runtime-candidate"))?;
        let tar = root.join("test-bin/tar");
        fs::write(&tar, "#!/bin/sh\nexit 0\n")?;
        crate::support::make_executable(&tar)?;
        let rsync = root.join("test-bin/rsync");
        fs::write(
            &rsync,
            "#!/bin/sh\nset -eu\nsource=${8:?source}\ndestination=${9:?destination}\nmkdir -p \"$destination\"\ncp -R \"${source%/}/.\" \"$destination/\"\n",
        )?;
        crate::support::make_executable(&rsync)?;
        run_git(&root, &["init", "-q"])?;
        run_git(&root, &["config", "user.email", "test@example.invalid"])?;
        run_git(&root, &["config", "user.name", "Candidate Fixture"])?;
        run_git(&root, &["add", "."])?;
        run_git(&root, &["commit", "-qm", "fixture"])?;
        let source_commit = String::from_utf8(run_git(&root, &["rev-parse", "HEAD"])?).map_err(|error| error.to_string())?;
        Ok(Self { temp, source_commit: source_commit.trim().into() })
    }

    fn assemble(&self) -> std::process::Output {
        Command::new("sh")
            .arg("scripts/assemble-runtime-candidate")
            .current_dir(self.root())
            .env("CANDIDATE_TAG", "runtime-candidate-test")
            .env("SOURCE_COMMIT", &self.source_commit)
            .env("GITHUB_RUN_ID", "1")
            .env("GITHUB_RUN_ATTEMPT", "1")
            .env("GITHUB_SERVER_URL", "https://github.invalid")
            .env("GITHUB_REPOSITORY", "example/codexy")
            .env("PATH", format!("{}:{}", self.root().join("test-bin").display(), std::env::var("PATH").expect("PATH")))
            .output()
            .expect("candidate assembly starts")
    }

    fn root(&self) -> &Path { self.temp.path() }
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(arguments).current_dir(root).output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("git {arguments:?} failed: {}", String::from_utf8_lossy(&output.stderr)).into())
    }
}
