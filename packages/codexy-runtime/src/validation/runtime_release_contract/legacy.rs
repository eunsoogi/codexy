pub(super) fn legacy_digest(platform: &str, server: &str) -> Option<&'static str> {
    match (platform, server) {
        ("darwin-arm64", "lsp") => {
            Some("0a6eda4597abd517f61c230aeabf6e81666e619aaeecc324275a2d26cdc70706")
        }
        ("darwin-arm64", "codegraph") => {
            Some("f6ac5faee4261167c7783e6cd69a0610b3cbf4abcbf5944d395213868d356dc6")
        }
        ("linux-x86_64", "lsp") => {
            Some("7504edd84efa75c346c478a6bff6076950b8339eaf95472a9754147ae6529663")
        }
        ("linux-x86_64", "codegraph") => {
            Some("218c5d896f912333c38c74034f6df6f0e54a70cf1fc418e1b04f808f29bea2b2")
        }
        _ => None,
    }
}
