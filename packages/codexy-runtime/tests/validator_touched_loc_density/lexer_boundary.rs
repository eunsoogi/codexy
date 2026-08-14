use super::*;

#[test]
fn touched_loc_tracks_rust_and_javascript_spans_without_header_leaks() -> TestResult {
    let rust = fixture("src/lib.rs", "fn readable() {}\n".to_owned())?;
    write(
        rust.path(),
        "src/lib.rs",
        "let fixture = r\"\nfirst(); second(); third();\n\";\n",
    )?;
    assert!(validate(rust.path())?.status.success());
    write(rust.path(), "src/lib.rs", "fn compact(){ let url=\"https://x\"; first(); second(); third(); }\n")?;
    assert!(!validate(rust.path())?.status.success());
    write(rust.path(), "src/lib.rs", "/* outer /* r\" */ still */ fn compact(){ first(); second(); third(); }\n")?;
    assert!(!validate(rust.path())?.status.success());
    write(rust.path(), "src/lib.rs", "fn readable() { let c = ';'; first(); }\n")?;
    assert!(validate(rust.path())?.status.success());
    let javascript = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(javascript.path(), "src/check.js", "for (let i = next(); i < limit; i++) { break; }\n")?;
    assert!(validate(javascript.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_selects_shebang_shell_and_quoted_awk_programs() -> TestResult {
    let shell = fixture("scripts/release", "#!/bin/sh\nexit 0\n".to_owned())?;
    write(shell.path(), "scripts/release", "#!/bin/sh\nfirst && second && third\n")?;
    assert!(!validate(shell.path())?.status.success());
    let awk = fixture("scripts/parser.sh", "exit 0\n".to_owned())?;
    write(awk.path(), "scripts/parser.sh", "awk -v mode='strict' 'BEGIN {\nfirst(); second(); third();\n}'\n")?;
    assert!(validate(awk.path())?.status.success());
    Ok(())
}
