const SHORT_OPTIONS: &str = "abcefhiklmnoprstuvxBCDEHOT";

pub(super) enum Invocation<'a> {
    NotShell,
    Command(&'a str),
    Safe,
    Invalid,
}

pub(super) fn invocation<'a>(tokens: &'a [&'a str]) -> Invocation<'a> {
    if !matches!(tokens.first(), Some(&"sh" | &"bash" | &"dash" | &"zsh")) {
        return Invocation::NotShell;
    }
    let mut rest = &tokens[1..];
    let mut command = false;
    while let Some((option, after)) = rest.split_first() {
        if *option == "--" {
            return if command {
                after
                    .first()
                    .map_or(Invocation::Invalid, |program| Invocation::Command(program))
            } else {
                Invocation::Safe
            };
        }
        if option.starts_with("--") {
            let Some(operands) = long_operands(option) else {
                return Invocation::Invalid;
            };
            let Some(next) = after.get(operands..) else {
                return Invocation::Invalid;
            };
            rest = next;
            continue;
        }
        if command && !option.starts_with(['-', '+']) {
            return Invocation::Command(option);
        }
        let Some(flags) = option
            .strip_prefix('-')
            .or_else(|| option.strip_prefix('+'))
        else {
            return Invocation::Invalid;
        };
        if flags.is_empty() || !flags.chars().all(|flag| SHORT_OPTIONS.contains(flag)) {
            return Invocation::Invalid;
        }
        let operands = flags
            .chars()
            .filter(|flag| matches!(flag, 'o' | 'O'))
            .count();
        let Some(next) = after.get(operands..) else {
            return Invocation::Invalid;
        };
        rest = next;
        command |= flags.contains('c');
    }
    Invocation::Invalid
}

fn long_operands(option: &str) -> Option<usize> {
    option
        .strip_prefix("--")?
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-')
        .then_some(matches!(option, "--rcfile" | "--init-file") as usize)
}
