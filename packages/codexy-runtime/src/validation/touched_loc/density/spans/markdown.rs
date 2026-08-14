pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut visible = Vec::new();
    let mut fence = None;
    let mut comment = false;
    for line in text.lines() {
        let line = without_comments(line, &mut comment);
        if let Some(active) = fence {
            if closes_fence(&line, active) {
                fence = None;
            }
            visible.push(None);
        } else if let Some(opening) = opens_fence(&line) {
            fence = Some(opening);
            visible.push(None);
        } else {
            visible.push(Some(line));
        }
    }
    hide_tables(&mut visible);
    visible
}

#[derive(Clone, Copy)]
struct Fence(char, usize);

fn without_comments(line: &str, comment: &mut bool) -> String {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        if *comment {
            let Some(end) = remainder.find("-->") else {
                return visible;
            };
            remainder = &remainder[end + 3..];
            *comment = false;
        }
        let Some(start) = remainder.find("<!--") else {
            visible.push_str(remainder);
            return visible;
        };
        visible.push_str(&remainder[..start]);
        remainder = &remainder[start + 4..];
        *comment = true;
    }
}

fn opens_fence(line: &str) -> Option<Fence> {
    let trimmed = trim_fence_indent(line)?;
    let marker = trimmed
        .chars()
        .next()
        .filter(|character| matches!(character, '`' | '~'))?;
    let count = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (count >= 3 && (marker != '`' || !trimmed[count..].contains('`')))
        .then_some(Fence(marker, count))
}

fn closes_fence(line: &str, Fence(marker, minimum): Fence) -> bool {
    let Some(trimmed) = trim_fence_indent(line) else {
        return false;
    };
    let count = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    count >= minimum && trimmed[count..].trim().is_empty()
}

fn trim_fence_indent(line: &str) -> Option<&str> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    (indent <= 3).then_some(&line[indent..])
}

fn hide_tables(visible: &mut [Option<String>]) {
    for index in 1..visible.len() {
        let Some(header) = visible[index - 1].as_deref().and_then(table_cells) else {
            continue;
        };
        let Some(delimiter) = visible[index].as_deref().and_then(table_cells) else {
            continue;
        };
        if header.len() != delimiter.len() || !delimiter.iter().all(|cell| delimiter_cell(cell)) {
            continue;
        }
        visible[index - 1] = None;
        visible[index] = None;
        for following in index + 1..visible.len() {
            if visible[following]
                .as_deref()
                .and_then(table_cells)
                .is_some_and(|cells| cells.len() == header.len())
            {
                visible[following] = None;
            } else {
                break;
            }
        }
    }
}

fn table_cells(line: &str) -> Option<Vec<String>> {
    let outer_pipes = line.trim().starts_with('|') && line.trim().ends_with('|');
    let mut cells = vec![String::new()];
    let mut escaped = false;
    let mut separators = 0;
    for character in line.trim().trim_matches('|').chars() {
        if escaped {
            cells.last_mut()?.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '|' {
            cells.push(String::new());
            separators += 1;
        } else {
            cells.last_mut()?.push(character);
        }
    }
    (separators > 0 || outer_pipes).then(|| {
        cells
            .into_iter()
            .map(|cell| cell.trim().to_owned())
            .collect()
    })
}

fn delimiter_cell(cell: &str) -> bool {
    let core = cell.trim_matches(':');
    core.len() >= 3 && core.chars().all(|character| character == '-')
}
