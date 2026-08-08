use std::path::Path;
use std::process::Command;

fn repository_root() -> &'static Path { codexy_runtime::paths::repository_root() }

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
    let manifest = repository.join("packages/codexy-runtime/Cargo.toml");
    let readme = repository.join("packages/codexy-runtime/README.md");

    assert!(readme.is_file(), "runtime package must own its Cargo README");
    let output = Command::new("cargo")
        .args([
            "package",
            "--manifest-path",
            manifest.to_str().ok_or("runtime manifest path")?,
            "--locked",
            "--allow-dirty",
            "--no-verify",
        ])
        .current_dir(&repository)
        .output()?;
    assert!(output.status.success(), "{output:?}");
    Ok(())
}
