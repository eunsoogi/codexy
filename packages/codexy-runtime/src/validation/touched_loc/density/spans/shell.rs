use std::collections::VecDeque;

use super::{
    shell_heredoc::{Heredoc, Span, spans},
    shell_projection::{Projection, continues},
};

pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut state = State {
        projection: Projection::new(),
        pending: VecDeque::new(),
        continued: None,
    };
    text.lines().map(|line| strip(line, &mut state)).collect()
}

struct State {
    projection: Projection,
    pending: VecDeque<Heredoc>,
    continued: Option<String>,
}

fn strip(line: &str, state: &mut State) -> Option<String> {
    if state
        .pending
        .front()
        .is_some_and(|heredoc| heredoc.terminates(line))
    {
        state.pending.pop_front();
        return None;
    }
    if !state.pending.is_empty() {
        return None;
    }
    let mut logical = state.continued.take().unwrap_or_default();
    if continues(line) {
        logical.push_str(&line[..line.len() - 1]);
        state.continued = Some(logical);
        return Some(String::new());
    }
    logical.push_str(line);
    let visible = state.projection.project(&logical);
    let heredocs = spans(&logical, &visible);
    if heredocs.is_empty() {
        return Some(visible);
    }
    let mut source = visible;
    for span in heredocs.iter().rev() {
        source.replace_range(span.start..span.end, "");
    }
    state.pending = heredocs.into_iter().map(Span::into_heredoc).collect();
    Some(source)
}
