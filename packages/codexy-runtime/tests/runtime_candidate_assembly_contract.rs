use std::fs;

#[path = "runtime_candidate_assembly_contract/fixture.rs"]
mod fixture;
#[path = "runtime_candidate_assembly_contract/heredoc.rs"]
mod heredoc;

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
    let root = codexy_runtime::paths::repository_root();
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

#[test]
fn candidate_assembly_removes_stale_repository_only_skills() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = CandidateFixture::new(FIRST_DECLARATION)?;
    let stale = ["plugin-marketplace-prep", "release-engineering"]
        .map(|skill| fixture.root().join(format!("dist/candidate/plugins/codexy/skills/{skill}/SKILL.md")));
    for path in &stale {
        fs::create_dir_all(path.parent().ok_or("stale skill parent missing")?)?;
        fs::write(path, "stale packaged skill\n")?;
    }

    let output = fixture.assemble();

    assert!(output.status.success(), "candidate assembly failed: {}", String::from_utf8_lossy(&output.stderr));
    for path in stale {
        assert!(!path.exists(), "candidate archive payload retained {path:?}");
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
fn candidate_assembly_rejects_compound_split_eval_overrides_and_ignores_inert_text()
-> Result<(), Box<dyn std::error::Error>> {
    for (line, succeeds) in [
        ("true && eval 'bundled_''platforms=darwin-arm64'", false),
        ("eval 'bundled_''platforms=darwin-arm64' && true", false),
        ("\"ev\"\"al\" 'bundled_''platforms=darwin-arm64'", false),
        ("command \"ev\"\"al\" 'bundled_''platforms=darwin-arm64'", false),
        ("runner=eval\n$runner 'bundled_''platforms=darwin-arm64'", false),
        ("true && runner=eval\n$runner 'bundled_''platforms=darwin-arm64'", false),
        ("runner=eval\n\"$runner\" 'bundled_''platforms=darwin-arm64'", false),
        ("runner=eval\n${runner} 'bundled_''platforms=darwin-arm64'", false),
        (
            "runner=eval\ncommand \"$runner\" 'bundled_''platforms=darwin-arm64'",
            false,
        ),
        ("runner=val\ne$runner 'bundled_''platforms=darwin-arm64'", false),
        (
            "runner=val\n\"e${runner}\" 'bundled_''platforms=darwin-arm64'",
            false,
        ),
        ("`printf eval` 'bundled_''platforms=darwin-arm64'", false),
        (
            "runner=eval\nbuiltin \"$runner\" 'bundled_''platforms=darwin-arm64'",
            false,
        ),
        ("printf '%s\\n' 'bundled_''platforms=darwin-arm64'", true),
        ("runner=eval\nprintf '%s\\n' \"$runner\"", true),
    ] {
        let fixture = CandidateFixture::new(&format!("{FIRST_DECLARATION}{line}\n"))?;
        assert_eq!(fixture.assemble().status.success(), succeeds, "{line}");
    }
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
