pub const BLOCK_SIZE: usize = 512;
pub const MODE: std::ops::Range<usize> = 100..108;
pub const SIZE: std::ops::Range<usize> = 124..136;
pub const CHECKSUM: std::ops::Range<usize> = 148..156;
pub const PREFIX: std::ops::Range<usize> = 345..500;

#[derive(serde::Serialize)]
pub struct GovernedWrapperHeaderEvidence {
    pub path: String,
    pub typeflag: u8,
    pub mode: String,
    pub checksum: String,
    pub calculated_checksum: usize,
    pub header_hex: String,
}

pub fn governed_wrapper_header_evidence(
    bytes: &[u8],
    wrappers: &[String],
) -> std::io::Result<Vec<GovernedWrapperHeaderEvidence>> {
    let mut evidence = Vec::new();
    let mut offset = 0;
    while let Some(header) = bytes.get(offset..offset + BLOCK_SIZE) {
        if header.iter().all(|byte| *byte == 0) {
            break;
        }
        let path = entry_path(header)?;
        let size = octal(&header[SIZE.clone()])?;
        if wrappers.binary_search(&path).is_ok() {
            evidence.push(GovernedWrapperHeaderEvidence {
                path,
                typeflag: header[156],
                mode: hex(&header[MODE.clone()]),
                checksum: hex(&header[CHECKSUM.clone()]),
                calculated_checksum: header_checksum(header),
                header_hex: hex(header),
            });
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
    Ok(evidence)
}

pub fn force_governed_wrapper_modes(
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

pub fn entry_path(header: &[u8]) -> std::io::Result<String> {
    let name = field(&header[..100])?;
    let prefix = field(&header[PREFIX])?;
    Ok(if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    })
}

pub fn field(value: &[u8]) -> std::io::Result<String> {
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    std::str::from_utf8(&value[..end])
        .map(str::to_owned)
        .map_err(|_| std::io::Error::other("tar header is not UTF-8"))
}

pub fn octal(value: &[u8]) -> std::io::Result<usize> {
    let text = std::str::from_utf8(value)
        .map_err(|_| std::io::Error::other("tar header number is not UTF-8"))?
        .trim_matches(['\0', ' ']);
    if text.is_empty() {
        return Ok(0);
    }
    usize::from_str_radix(text, 8)
        .map_err(|_| std::io::Error::other("tar header number is invalid"))
}

pub fn write_octal(value: &mut [u8], number: usize) -> std::io::Result<()> {
    let digits = value.len() - 1;
    let text = format!("{number:0digits$o}");
    if text.len() != digits {
        return Err(std::io::Error::other("tar header number does not fit"));
    }
    value[..digits].copy_from_slice(text.as_bytes());
    value[digits] = 0;
    Ok(())
}

pub fn write_checksum(header: &mut [u8]) -> std::io::Result<()> {
    header[CHECKSUM.clone()].fill(b' ');
    let checksum = header_checksum(header);
    write_octal(&mut header[CHECKSUM], checksum)
}

pub fn header_checksum(header: &[u8]) -> usize {
    header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            usize::from(if CHECKSUM.contains(&index) {
                b' '
            } else {
                *byte
            })
        })
        .sum()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
