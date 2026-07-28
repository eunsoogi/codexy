use std::{fs, path::Path};

#[path = "runtime_candidate_assembly_contract/fixture.rs"]
mod fixture;

use fixture::CandidateFixture;

const FIRST_DECLARATION: &str = "bundled_platforms=\"darwin-arm64 linux-x86_64\"\n";
const ACTIVATED_DECLARATION: &str = "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"\n";

#[test]
fn candidate_assembly_accepts_first_and_subsequent_truthful_wrapper_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    for declaration in [FIRST_DECLARATION, ACTIVATED_DECLARATION] {
        let wrapper = format!(
            "{declaration}case \" $bundled_platforms \" in\n  *\" linux-x86_64 \"*) ;;\nesac\nprintf '%s\\n' \"${{bundled_platforms}}\"\n"
        );
        let fixture = CandidateFixture::new(&wrapper)?;
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
            assert_eq!(
                wrapper.replace("\r\n", "\n"),
                wrapper_platform_reads(&ACTIVATED_DECLARATION)
            );
        }
    }
    Ok(())
}

#[test]
fn candidate_assembly_accepts_each_real_wrapper_body()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for server in ["lsp", "codegraph"] {
        let wrapper = fs::read_to_string(root.join("plugins/codexy/mcp").join(format!("codexy-mcp-{server}")))?;
        let fixture = CandidateFixture::new(&wrapper)?;
        let output = fixture.assemble();
        assert!(
            output.status.success(),
            "candidate assembly rejected codexy-mcp-{server}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expected = wrapper.replacen(FIRST_DECLARATION, ACTIVATED_DECLARATION, 1);
        for output_server in ["lsp", "codegraph"] {
            assert_eq!(
                fs::read(
                    fixture
                        .root()
                        .join("dist/candidate/plugins/codexy/mcp")
                        .join(format!("codexy-mcp-{output_server}")),
                )?,
                expected.as_bytes()
            );
        }
    }
    Ok(())
}

fn wrapper_platform_reads(declaration: &str) -> String {
    format!(
        "{declaration}case \" $bundled_platforms \" in\n  *\" linux-x86_64 \"*) ;;\nesac\nprintf '%s\\n' \"${{bundled_platforms}}\"\n"
    )
}

#[test]
fn candidate_assembly_preserves_wrapper_bytes_while_rewriting_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    for declaration in [
        "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
        "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
    ] {
        for (separator, ending) in [("\n", "\n"), ("\r\n", "\r\n"), ("\n", "")] {
            let wrapper = format!("#!/bin/sh{separator}set -eu{separator}{declaration}{ending}");
            let fixture = CandidateFixture::new(&wrapper)?;
            let output = fixture.assemble();
            assert!(
                output.status.success(),
                "candidate assembly failed for {wrapper:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let expected = wrapper.replacen(
                declaration,
                "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
                1,
            );
            for server in ["lsp", "codegraph"] {
                let wrapper = fs::read(
                    fixture
                        .root()
                        .join("dist/candidate/plugins/codexy/mcp")
                        .join(format!("codexy-mcp-{server}")),
                )?;
                assert_eq!(wrapper, expected.as_bytes());
            }
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
        format!("{FIRST_DECLARATION}  bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}export bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}:; bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}unset bundled_platforms\n"),
        format!("{FIRST_DECLARATION}read bundled_platforms\n"),
        format!("{FIRST_DECLARATION}local bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}typeset bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}declare bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"),
        format!("{FIRST_DECLARATION}${{bundled_platforms:=plan9-mips64}}\n"),
        format!("{FIRST_DECLARATION}eval 'bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"'\n"),
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

#[test]
fn candidate_assembly_rejects_readonly_shell_overrides()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!(
        "{FIRST_DECLARATION}readonly bundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\n"
    ))?;
    let output = fixture.assemble();
    assert!(!output.status.success(), "candidate assembly accepted readonly override");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("candidate wrapper platform declaration mismatch")
    );
    Ok(())
}

#[test]
fn candidate_assembly_rejects_declarations_inside_heredocs()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!("cat <<'WRAPPER'\n{FIRST_DECLARATION}WRAPPER\n"))?;
    let output = fixture.assemble();
    assert!(!output.status.success(), "candidate assembly accepted heredoc declaration");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("candidate wrapper platform declaration mismatch")
    );
    Ok(())
}

#[test]
fn candidate_assembly_rejects_declarations_inside_word_adjacent_comment_heredocs()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!(
        ": foo#bar <<'WRAPPER'\n{FIRST_DECLARATION}WRAPPER\n"
    ))?;
    let output = fixture.assemble();
    assert!(
        !output.status.success(),
        "candidate assembly accepted declaration inside word-adjacent-comment heredoc"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("candidate wrapper platform declaration mismatch")
    );
    Ok(())
}

#[test]
fn candidate_assembly_ignores_inert_heredoc_declaration_text()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!(
        "{FIRST_DECLARATION}cat <<'WRAPPER'\nbundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\nWRAPPER\n"
    ))?;
    let output = fixture.assemble();
    assert!(
        output.status.success(),
        "candidate assembly rejected inert heredoc text: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn candidate_assembly_ignores_word_adjacent_comment_heredoc_text()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!(
        "{FIRST_DECLARATION}: foo#bar <<'WRAPPER'\nbundled_platforms=\"darwin-arm64 linux-x86_64 plan9-mips64\"\nWRAPPER\n"
    ))?;
    let output = fixture.assemble();
    assert!(
        output.status.success(),
        "candidate assembly rejected inert word-adjacent-comment heredoc text: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn candidate_assembly_rejects_unterminated_heredocs()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(&format!("{FIRST_DECLARATION}cat <<'WRAPPER'\n"))?;
    let output = fixture.assemble();
    assert!(!output.status.success(), "candidate assembly accepted unterminated heredoc");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("candidate wrapper platform declaration mismatch")
    );
    Ok(())
}
