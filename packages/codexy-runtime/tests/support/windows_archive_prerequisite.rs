pub(crate) fn assert_windows_prerequisite_contract(text: &str) {
    super::release_archive::assert_structured_literals(
        text,
        "Windows archive scanner prerequisite",
        &[
            "Get-Command grep",
            "Git\\usr\\bin",
            "msys64\\usr\\bin",
            "grep.exe",
            "GITHUB_PATH",
        ],
    );
    super::release_archive::assert_structured_absent_literals(
        text,
        "Windows archive scanner prerequisite",
        &["choco install ripgrep"],
    );
}
