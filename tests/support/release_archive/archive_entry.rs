const BLOCK_SIZE: usize = 512;
const MODE: std::ops::Range<usize> = 100..108;
const SIZE: std::ops::Range<usize> = 124..136;
const CHECKSUM: std::ops::Range<usize> = 148..156;
const PREFIX: std::ops::Range<usize> = 345..500;

pub(super) fn force_governed_wrapper_modes(
    archive: &std::path::Path,
    wrappers: &[String],
) -> std::io::Result<()> {
    let mut bytes = std::fs::read(archive)?;
    let mut found = std::collections::BTreeSet::new();
    let mut offset = 0;
    while let Some(header) = bytes.get_mut(offset..offset + BLOCK_SIZE) {
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        let path = entry_path(header)?;
        let size = octal(&header[SIZE.clone()])?;
        if wrappers.binary_search(&path).is_ok() {
            let source_mode = u32::try_from(octal(&header[MODE.clone()])?)
                .map_err(|_| std::io::Error::other("tar mode does not fit"))?;
            let mode = super::governed_archive_mode(true, true, source_mode)
                .ok_or_else(|| std::io::Error::other("Windows wrapper archive mode unavailable"))?;
            write_octal(
                &mut header[MODE.clone()],
                usize::try_from(mode)
                    .map_err(|_| std::io::Error::other("tar mode does not fit"))?,
            )?;
            write_checksum(header)?;
            found.insert(path);
        }
        let padded = size
            .checked_add(BLOCK_SIZE - 1)
            .ok_or_else(|| std::io::Error::other("tar entry size overflow"))?
            / BLOCK_SIZE
            * BLOCK_SIZE;
        offset = offset
            .checked_add(BLOCK_SIZE + padded)
            .ok_or_else(|| std::io::Error::other("tar archive offset overflow"))?;
    }
    if found.len() != wrappers.len() {
        return Err(std::io::Error::other(
            "governed wrapper missing from archive",
        ));
    }
    std::fs::write(archive, bytes)
}

fn entry_path(header: &[u8]) -> std::io::Result<String> {
    let name = field(&header[..100])?;
    let prefix = field(&header[PREFIX])?;
    Ok(if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    })
}

fn field(value: &[u8]) -> std::io::Result<String> {
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    std::str::from_utf8(&value[..end])
        .map(str::to_owned)
        .map_err(|_| std::io::Error::other("tar header is not UTF-8"))
}

fn octal(value: &[u8]) -> std::io::Result<usize> {
    let text = std::str::from_utf8(value)
        .map_err(|_| std::io::Error::other("tar header number is not UTF-8"))?
        .trim_matches(['\0', ' ']);
    if text.is_empty() {
        return Ok(0);
    }
    usize::from_str_radix(text, 8)
        .map_err(|_| std::io::Error::other("tar header number is invalid"))
}

fn write_octal(value: &mut [u8], number: usize) -> std::io::Result<()> {
    let digits = value.len() - 1;
    let text = format!("{number:0digits$o}");
    if text.len() != digits {
        return Err(std::io::Error::other("tar header number does not fit"));
    }
    value[..digits].copy_from_slice(text.as_bytes());
    value[digits] = 0;
    Ok(())
}

fn write_checksum(header: &mut [u8]) -> std::io::Result<()> {
    header[CHECKSUM.clone()].fill(b' ');
    let checksum = header.iter().map(|byte| usize::from(*byte)).sum();
    write_octal(&mut header[CHECKSUM], checksum)
}

#[cfg(test)]
mod tests {
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
}
