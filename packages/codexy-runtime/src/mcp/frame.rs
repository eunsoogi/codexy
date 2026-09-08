use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use super::{MAX_BUFFER_BYTES, MAX_FRAME_BYTES, MAX_HEADER_BYTES};

#[derive(Debug, Default)]
pub(crate) struct FrameParser {
    buffer: Vec<u8>,
}

impl FrameParser {
    pub(crate) fn extend(&mut self, chunk: &[u8]) -> Result<()> {
        if self.buffer.len().saturating_add(chunk.len()) > MAX_BUFFER_BYTES {
            bail!("MCP input exceeds the frame size limit");
        }
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }

    pub(crate) fn next_frame(&mut self) -> Result<Option<Value>> {
        if let Some(header_end) = find_header_end(&self.buffer) {
            let header = std::str::from_utf8(&self.buffer[..header_end])
                .context("MCP header is not UTF-8")?;
            if header_has_content_length(header) {
                return self.next_content_length_frame();
            }
        } else if starts_like_header_block(&self.buffer) {
            return Ok(None);
        }
        if starts_with_content_length(&self.buffer) {
            return self.next_content_length_frame();
        }
        self.next_newline_frame()
    }

    fn next_content_length_frame(&mut self) -> Result<Option<Value>> {
        let Some(header_end) = find_header_end(&self.buffer) else {
            if self.buffer.len() > MAX_HEADER_BYTES {
                bail!("MCP header exceeds the size limit");
            }
            return Ok(None);
        };
        if header_end > MAX_HEADER_BYTES {
            bail!("MCP header exceeds the size limit");
        }
        let header =
            std::str::from_utf8(&self.buffer[..header_end]).context("MCP header is not UTF-8")?;
        let length = content_length(header)?;
        let start = header_end + 4;
        if length > MAX_FRAME_BYTES {
            bail!("MCP frame exceeds the size limit");
        }
        let end = start
            .checked_add(length)
            .context("MCP frame length overflows the input buffer")?;
        if self.buffer.len() < end {
            return Ok(None);
        }
        let body = self.buffer[start..end].to_vec();
        self.buffer.drain(..end);
        let text = std::str::from_utf8(&body).context("MCP JSON frame is not UTF-8")?;
        crate::strict_json::parse(text)
            .map(Some)
            .context("parsing MCP JSON frame")
    }

    fn next_newline_frame(&mut self) -> Result<Option<Value>> {
        let Some(line_end) = self.buffer.iter().position(|byte| *byte == b'\n') else {
            if self.buffer.len() > MAX_FRAME_BYTES {
                bail!("MCP newline frame exceeds the size limit");
            }
            return Ok(None);
        };
        if line_end > MAX_FRAME_BYTES {
            bail!("MCP newline frame exceeds the size limit");
        }
        let mut line = self.buffer.drain(..=line_end).collect::<Vec<_>>();
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        if line.is_empty() {
            return Ok(None);
        }
        let text = std::str::from_utf8(&line).context("MCP newline JSON is not UTF-8")?;
        crate::strict_json::parse(text)
            .map(Some)
            .context("parsing MCP newline JSON message")
    }
}

fn starts_with_content_length(buffer: &[u8]) -> bool {
    const HEADER: &[u8] = b"content-length:";
    buffer.len() >= HEADER.len()
        && buffer[..HEADER.len()]
            .iter()
            .zip(HEADER)
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
}

fn starts_like_header_block(buffer: &[u8]) -> bool {
    let first_line_end = buffer
        .iter()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .unwrap_or(buffer.len());
    let first_line = &buffer[..first_line_end];
    let Some(colon) = first_line.iter().position(|byte| *byte == b':') else {
        return false;
    };
    let name = &first_line[..colon];
    !name.is_empty()
        && name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

fn header_has_content_length(header: &str) -> bool {
    header.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(header: &str) -> Result<usize> {
    let mut found = None;
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            if found.is_some() {
                bail!("duplicate Content-Length header");
            }
            found = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .context("parsing Content-Length header")?,
            );
        }
    }
    found.context("Missing Content-Length header")
}

#[cfg(test)]
mod tests {
    use super::{FrameParser, MAX_FRAME_BYTES};
    use serde_json::json;

    #[test]
    fn parser_rejects_oversized_newline_frame_before_buffer_growth() {
        let mut parser = FrameParser::default();
        assert!(parser.extend(&vec![b'x'; MAX_FRAME_BYTES + 1]).is_err());
    }

    #[test]
    fn parser_rejects_oversized_content_length() {
        let mut parser = FrameParser::default();
        parser
            .extend(b"Content-Length: 1048577\r\n\r\n")
            .expect("header fits");
        assert!(parser.next_frame().is_err());
    }

    #[test]
    fn parser_accepts_a_cancel_notification_as_a_regular_json_message() {
        let mut parser = FrameParser::default();
        let mut message = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": { "requestId": 4 }
        }))
        .expect("serialize");
        message.push(b'\n');
        parser.extend(&message).expect("message fits");
        let parsed = parser.next_frame().expect("parse").expect("message");
        assert_eq!(parsed["method"], "notifications/cancelled");
    }
}
