pub(crate) fn x86_64_executable() -> Vec<u8> {
    let mut bytes = vec![0; 4096];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&(0x80_u32).to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    bytes[0x84..0x86].copy_from_slice(&0x8664_u16.to_le_bytes());
    bytes[0x86..0x88].copy_from_slice(&1_u16.to_le_bytes());
    bytes[0x94..0x96].copy_from_slice(&0xf0_u16.to_le_bytes());
    bytes[0x96..0x98].copy_from_slice(&0x0022_u16.to_le_bytes());
    bytes[0x98..0x9a].copy_from_slice(&0x20b_u16.to_le_bytes());
    bytes
}
