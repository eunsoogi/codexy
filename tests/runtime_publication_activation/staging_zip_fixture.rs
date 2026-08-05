use std::{fs, io, path::Path};

pub(super) fn write_receipt_archive(path: &Path, receipt: &[u8]) -> io::Result<()> {
    let name = b"runtime-staging-receipt.json";
    let name_length = u16::try_from(name.len()).map_err(|_| io::Error::other("fixture name"))?;
    let size = u32::try_from(receipt.len()).map_err(|_| io::Error::other("fixture receipt"))?;
    let checksum = crc32(receipt);
    let mut archive = Vec::with_capacity(name.len() * 2 + receipt.len() + 98);
    local_header(&mut archive, name_length, size, checksum);
    archive.extend_from_slice(name);
    archive.extend_from_slice(receipt);
    let directory_offset = u32::try_from(archive.len()).map_err(|_| io::Error::other("fixture offset"))?;
    central_header(&mut archive, name_length, size, checksum);
    archive.extend_from_slice(name);
    let directory_size = u32::try_from(archive.len())
        .map_err(|_| io::Error::other("fixture directory"))?
        .checked_sub(directory_offset)
        .ok_or_else(|| io::Error::other("fixture directory order"))?;
    archive.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    archive.extend_from_slice(&[0; 4]);
    archive.extend_from_slice(&1_u16.to_le_bytes());
    archive.extend_from_slice(&1_u16.to_le_bytes());
    archive.extend_from_slice(&directory_size.to_le_bytes());
    archive.extend_from_slice(&directory_offset.to_le_bytes());
    archive.extend_from_slice(&0_u16.to_le_bytes());
    fs::write(path, archive)
}

fn local_header(archive: &mut Vec<u8>, name_length: u16, size: u32, checksum: u32) {
    archive.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    archive.extend_from_slice(&20_u16.to_le_bytes());
    archive.extend_from_slice(&[0; 8]);
    archive.extend_from_slice(&checksum.to_le_bytes());
    archive.extend_from_slice(&size.to_le_bytes());
    archive.extend_from_slice(&size.to_le_bytes());
    archive.extend_from_slice(&name_length.to_le_bytes());
    archive.extend_from_slice(&0_u16.to_le_bytes());
}

fn central_header(archive: &mut Vec<u8>, name_length: u16, size: u32, checksum: u32) {
    archive.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    archive.extend_from_slice(&20_u16.to_le_bytes());
    archive.extend_from_slice(&20_u16.to_le_bytes());
    archive.extend_from_slice(&[0; 8]);
    archive.extend_from_slice(&checksum.to_le_bytes());
    archive.extend_from_slice(&size.to_le_bytes());
    archive.extend_from_slice(&size.to_le_bytes());
    archive.extend_from_slice(&name_length.to_le_bytes());
    archive.extend_from_slice(&[0; 16]);
}

fn crc32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(!0_u32, |checksum, byte| {
        (0..8).fold(checksum ^ u32::from(*byte), |checksum, _| {
            (checksum >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(checksum & 1))
        })
    }) ^ !0_u32
}

#[test]
fn receipt_archive_has_a_single_stored_receipt_entry() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = temp.path().join("receipt.zip");
    write_receipt_archive(&archive, b"receipt")?;
    let bytes = fs::read(archive)?;
    assert_eq!(&bytes[..4], b"PK\x03\x04");
    assert_eq!(&bytes[30..58], b"runtime-staging-receipt.json");
    assert_eq!(&bytes[58..65], b"receipt");
    assert_eq!(&bytes[65..69], b"PK\x01\x02");
    assert_eq!(&bytes[139..143], b"PK\x05\x06");
    Ok(())
}
