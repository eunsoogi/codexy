use super::{CandidateFixture, FIRST_DECLARATION};

const ESCAPED_WORD_HEREDOC_PREFIXES: [&str; 3] = [
    ": foo\\ #bar <<'WRAPPER'",
    ": foo\\;#bar <<'WRAPPER'",
    ": foo\\\n#bar <<'WRAPPER'",
];

#[test]
fn candidate_assembly_rejects_declarations_inside_escaped_word_heredocs()
-> Result<(), Box<dyn std::error::Error>> {
    for prefix in ESCAPED_WORD_HEREDOC_PREFIXES {
        let fixture = CandidateFixture::new(&format!("{prefix}\n{FIRST_DECLARATION}WRAPPER\n"))?;
        let output = fixture.assemble();
        assert!(
            !output.status.success(),
            "candidate assembly accepted declaration inside escaped-word heredoc {prefix:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("candidate wrapper platform declaration mismatch")
        );
    }
    Ok(())
}

#[test]
fn candidate_assembly_ignores_escaped_word_heredoc_declaration_text()
-> Result<(), Box<dyn std::error::Error>> {
    for prefix in ESCAPED_WORD_HEREDOC_PREFIXES {
        let fixture = CandidateFixture::new(&format!(
            "{FIRST_DECLARATION}{prefix}\nbundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\nWRAPPER\n"
        ))?;
        let output = fixture.assemble();
        assert!(
            output.status.success(),
            "candidate assembly rejected inert escaped-word heredoc text {prefix:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn candidate_assembly_accepts_hyphenated_heredoc_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!(
        "{FIRST_DECLARATION}cat <<'EOF-1'\nbody\nEOF-1\n"
    ))?;
    let output = fixture.assemble();
    assert!(
        output.status.success(),
        "candidate assembly rejected a valid hyphenated heredoc delimiter: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
