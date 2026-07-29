const SHORT_OPTIONS: &str = "abcefhiklmnoprstuvxBCDEHOT";

pub(super) fn program<'a>(tokens: &'a [&'a str]) -> Option<&'a str> {
    if !matches!(tokens.first(), Some(&"sh" | &"bash" | &"dash" | &"zsh")) {
        return None;
    }
    let mut rest = &tokens[1..];
    let mut command = false;
    while let Some((option, after)) = rest.split_first() {
        if *option == "--" {
            return command.then(|| after.first().copied()).flatten();
        }
        if option.starts_with("--") {
            rest = after.get(long_operands(option)?..)?;
            continue;
        }
        if command && !option.starts_with(['-', '+']) {
            return Some(*option);
        }
        let flags = option
            .strip_prefix('-')
            .or_else(|| option.strip_prefix('+'))?;
        if flags.is_empty() || !flags.chars().all(|flag| SHORT_OPTIONS.contains(flag)) {
            return None;
        }
        let operands = flags
            .chars()
            .filter(|flag| matches!(flag, 'o' | 'O'))
            .count();
        rest = after.get(operands..)?;
        command |= flags.contains('c');
    }
    None
}

fn long_operands(option: &str) -> Option<usize> {
    match option {
        "--rcfile" | "--init-file" => Some(1),
        "--debugger" | "--login" | "--noediting" | "--noprofile" | "--norc" | "--posix"
        | "--restricted" => Some(0),
        _ => None,
    }
}
