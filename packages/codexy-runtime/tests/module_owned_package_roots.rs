use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|candidate| candidate.join("AGENTS.md").is_file())
        .map(Path::to_path_buf)
        .expect("locate repository root")
}

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
