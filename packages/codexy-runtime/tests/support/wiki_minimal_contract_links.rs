use std::ops::Range;

use crate::support::wiki_minimal_contract_parser::active_link_lines;

pub(crate) fn markdown_link_count(text: &str, label: &str, target: &str) -> Result<usize, String> {
    Ok(active_link_lines(text)?
        .iter()
        .map(|line| links_in_line(line, label, target))
        .sum())
}

fn links_in_line(line: &str, label: &str, target: &str) -> usize {
    let bytes = line.as_bytes();
    let mut count = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
        } else if bytes[index] == b'[' {
            if let Some(node) = link_node(line, index) {
                if !node.image && &line[node.label] == label && &line[node.destination] == target {
                    count += 1;
                }
                index = node.end;
            } else {
                index += 1;
            }
        } else {
            index += 1;
        }
    }
    count
}

struct LinkNode {
    image: bool,
    label: Range<usize>,
    destination: Range<usize>,
    end: usize,
}

fn link_node(line: &str, start: usize) -> Option<LinkNode> {
    let label_start = start + 1;
    let label_end = matching(line, label_start, b'[', b']')?;
    if line.as_bytes().get(label_end + 1) != Some(&b'(') {
        return None;
    }
    let destination_start = label_end + 2;
    let (destination_end, end) = destination(line, destination_start)?;
    Some(LinkNode {
        image: start > 0
            && line.as_bytes()[start - 1] == b'!'
            && !escaped(line.as_bytes(), start - 1),
        label: label_start..label_end,
        destination: destination_start..destination_end,
        end,
    })
}

fn matching(line: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut depth = 1;
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
        } else if bytes[index] == open {
            depth += 1;
            index += 1;
        } else if bytes[index] == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
            index += 1;
        } else {
            index += 1;
        }
    }
    None
}

fn destination(line: &str, start: usize) -> Option<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut depth = 1;
    let mut quote = None;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index += 2;
        } else if quote == Some(byte) {
            quote = None;
            index += 1;
        } else if quote.is_some() {
            index += 1;
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            index += 1;
        } else if byte == b'(' {
            depth += 1;
            index += 1;
        } else if byte == b')' {
            depth -= 1;
            if depth == 0 {
                return Some((index, index + 1));
            }
            index += 1;
        } else {
            index += 1;
        }
    }
    None
}

fn escaped(bytes: &[u8], index: usize) -> bool {
    bytes[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}
