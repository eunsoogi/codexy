use std::path::PathBuf;

pub(super) fn decode_local_file_uri(uri: &str) -> Result<PathBuf, String> {
    let encoded_path = uri
        .strip_prefix("file:///")
        .ok_or_else(|| format!("expected local file URI, got {uri}"))?;
    let decoded = percent_decode(encoded_path)?;
    let decoded = if decoded.as_bytes().get(1) == Some(&b':') {
        decoded
    } else if decoded.starts_with('/')
        && decoded.as_bytes().get(2) == Some(&b':')
    {
        decoded[1..].to_owned()
    } else {
        format!("/{decoded}")
    };
    Ok(PathBuf::from(decoded))
}

fn percent_decode(input: &str) -> Result<String, String> {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(bytes[index]);
            index += 1;
            continue;
        }
        let high = *bytes
            .get(index + 1)
            .ok_or_else(|| format!("incomplete percent escape in {input}"))?;
        let low = *bytes
            .get(index + 2)
            .ok_or_else(|| format!("incomplete percent escape in {input}"))?;
        let byte = (hex_value(high, input)? << 4) | hex_value(low, input)?;
        output.push(byte);
        index += 3;
    }
    String::from_utf8(output).map_err(|error| error.to_string())
}

fn hex_value(byte: u8, input: &str) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid percent escape in {input}")),
    }
}
