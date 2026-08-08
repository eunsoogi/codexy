use super::*;

fn entry(path: &str, mode: usize, contents: &[u8]) -> Vec<u8> {
    let mut header = [0_u8; BLOCK_SIZE];
    header[..path.len()].copy_from_slice(path.as_bytes());
    write_octal(&mut header[MODE.clone()], mode).expect("mode");
    write_octal(&mut header[SIZE.clone()], contents.len()).expect("size");
    write_checksum(&mut header).expect("checksum");
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(contents);
    bytes.resize(
        BLOCK_SIZE + contents.len().div_ceil(BLOCK_SIZE) * BLOCK_SIZE,
        0,
    );
    bytes
}

fn mode(bytes: &[u8], offset: usize) -> usize {
    octal(&bytes[offset + MODE.start..offset + MODE.end]).expect("mode")
}

#[test]
fn governed_wrapper_mode_rewrite_preserves_other_entries_and_binary_contents() {
    let root = tempfile::tempdir().expect("tempdir");
    let archive = root.path().join("fixture.tar");
    let wrapper = "plugins/codexy/mcp/codexy-mcp-lsp".to_owned();
    let other = entry("plugins/codexy/assets/binary", 0o644, b"\0binary\xff");
    let wrapper_entry = entry(&wrapper, 0o644, b"#!/bin/sh\necho fixture\n");
    let mut bytes = other.clone();
    bytes.extend_from_slice(&wrapper_entry);
    bytes.extend_from_slice(&[0; BLOCK_SIZE * 2]);
    std::fs::write(&archive, &bytes).expect("archive");

    force_governed_wrapper_modes(&archive, &[wrapper.clone()]).expect("rewrite");
    let rewritten = std::fs::read(&archive).expect("rewritten archive");
    assert_eq!(mode(&rewritten, 0), 0o644);
    assert_eq!(mode(&rewritten, other.len()), 0o755);
    assert_eq!(&rewritten[..MODE.start], &bytes[..MODE.start]);
    assert_eq!(
        &rewritten[BLOCK_SIZE..other.len()],
        &bytes[BLOCK_SIZE..other.len()]
    );
    assert_eq!(
        &rewritten[other.len() + BLOCK_SIZE..],
        &bytes[other.len() + BLOCK_SIZE..]
    );

    let duplicate = root.path().join("duplicate.tar");
    std::fs::write(&duplicate, &bytes).expect("duplicate archive");
    force_governed_wrapper_modes(&duplicate, &[wrapper]).expect("duplicate rewrite");
    assert_eq!(
        std::fs::read(&duplicate).expect("duplicate bytes"),
        rewritten
    );
}

#[test]
fn governed_wrapper_mode_rewrite_fails_when_a_requested_entry_is_absent() {
    let root = tempfile::tempdir().expect("tempdir");
    let archive = root.path().join("fixture.tar");
    std::fs::write(
        &archive,
        [
            entry("plugins/codexy/README.md", 0o644, b"text"),
            vec![0; BLOCK_SIZE * 2],
        ]
        .concat(),
    )
    .expect("archive");
    assert!(
        force_governed_wrapper_modes(&archive, &["plugins/codexy/mcp/codexy-mcp-lsp".into()])
            .is_err()
    );
}

#[test]
fn governed_wrapper_header_evidence_keeps_the_complete_raw_header() {
    let wrapper = "plugins/codexy/mcp/codexy-mcp-lsp";
    let bytes = entry(wrapper, 0o644, b"#!/bin/sh\necho fixture\n");

    let evidence = governed_wrapper_header_evidence(&bytes, &[wrapper.into()]).expect("evidence");

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].path, wrapper);
    assert_eq!(evidence[0].mode, "3030303036343400");
    assert_eq!(evidence[0].header_hex.len(), BLOCK_SIZE * 2);
}
