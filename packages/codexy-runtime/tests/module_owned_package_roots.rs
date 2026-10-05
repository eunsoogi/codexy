use std::path::Path;
use std::process::Command;

fn repository_root() -> &'static Path { codexy_runtime::paths::repository_root() }

// Keep Cargo's VCS scan away from the checkout's target directory: sibling system tests can
// re-enter Cargo and create compiler scratch directories there while this package smoke runs.
fn copy_package_source(source: &Path, destination: &Path, package_root: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if package_root && name == "target" {
            continue;
        }

        let source_path = entry.path();
        let destination_path = destination.join(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_package_source(&source_path, &destination_path, false)?;
        } else if kind.is_file() {
            std::fs::copy(source_path, destination_path)?;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("unsupported runtime package source entry: {}", source_path.display()),
            ));
        }
    }
    Ok(())
}

// Cargo must find package metadata and toolchain files beside the runtime sources, not at workspace root.
#[test]
fn rust_runtime_is_a_module_owned_package_root() {
    let repository = repository_root();
    let runtime = repository.join("packages/codexy-runtime");

    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "rustfmt.toml",
        "clippy.toml",
    ] {
        assert!(
            runtime.join(file).is_file(),
            "missing runtime package {file}"
        );
        assert!(
            !repository.join(file).exists(),
            "repository root must not retain {file}"
        );
    }

    assert!(runtime.join("src").is_dir());
    assert!(runtime.join("tests").is_dir());
    assert!(
        repository
            .join("packages/getcodexy/pyproject.toml")
            .is_file()
    );
}

#[test]
fn runtime_package_has_a_local_readme_and_can_be_packaged() -> Result<(), Box<dyn std::error::Error>> {
    let repository = repository_root();
    let runtime = repository.join("packages/codexy-runtime");
    let readme = runtime.join("README.md");

    assert!(readme.is_file(), "runtime package must own its Cargo README");
    let temporary = tempfile::tempdir()?;
    let package_snapshot = temporary.path().join("codexy-runtime");
    copy_package_source(&runtime, &package_snapshot, true)?;
    assert!(
        !package_snapshot.join("target").exists(),
        "package smoke snapshot must not include live compiler artifacts"
    );
    let manifest = package_snapshot.join("Cargo.toml");

    let output = Command::new("cargo")
        .args([
            "package",
            "--manifest-path",
            manifest.to_str().ok_or("runtime manifest path")?,
            "--locked",
            "--allow-dirty",
            "--no-verify",
        ])
        .current_dir(temporary.path())
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
