use std::collections::BTreeMap;

const PUBLISH: &str = "publish_version_pr_metadata";

pub(super) fn validate_version_pr_adapter(adapter: &str) -> Result<(), String> {
    let shell = ShellStep::parse(adapter)?;
    validate_publisher(&shell)?;
    validate_transaction(&shell)
}

struct ShellStep<'a> {
    functions: BTreeMap<&'a str, Vec<&'a str>>,
    top_level: Vec<&'a str>,
}

impl<'a> ShellStep<'a> {
    fn parse(run: &'a str) -> Result<Self, String> {
        let lines = run.lines().collect::<Vec<_>>();
        let mut functions = BTreeMap::new();
        let mut top_level = Vec::new();
        let mut index = 0;
        while index < lines.len() {
            if let Some(name) = function_name(lines[index]) {
                let start = index + 1;
                index = start;
                while index < lines.len() && lines[index].trim() != "}" {
                    index += 1;
                }
                if index == lines.len() {
                    return Err(format!("unterminated function {name}"));
                }
                if functions.insert(name, lines[start..index].to_vec()).is_some() {
                    return Err(format!("duplicate function {name}"));
                }
            } else {
                top_level.push(lines[index]);
            }
            index += 1;
        }
        Ok(Self { functions, top_level })
    }
}

fn function_name(line: &str) -> Option<&str> {
    let declaration = line.trim().strip_suffix('{')?.trim_end();
    let name = declaration
        .strip_suffix("()")
        .or_else(|| declaration.strip_suffix(" ()"))?
        .trim_end();
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then_some(name)
}

fn validate_publisher(shell: &ShellStep<'_>) -> Result<(), String> {
    let body = shell
        .functions
        .get(PUBLISH)
        .ok_or_else(|| format!("missing {PUBLISH} function"))?;
    let commands = logical_commands(body);
    let render = command_position(&commands, |command| {
        command == "render_version_pr_metadata \"$phase\""
    })?;
    let labels = command_position(&commands, |command| {
        command.starts_with("gh api --method PUT ")
            && command.contains("repos/$GITHUB_REPOSITORY/issues/$pr_number/labels")
            && command.contains("--input \"$state_dir/metadata/labels.json\"")
    })?;
    let edit = command_position(&commands, |command| {
        command.starts_with("gh pr edit ")
            && command.contains("--title \"$title\"")
            && command.contains("--body-file \"$state_dir/metadata/body.md\"")
    })?;
    ordered(&[("render", render), ("label mutation", labels), ("PR edit", edit)])?;
    for (name, body) in &shell.functions {
        if *name == PUBLISH {
            continue;
        }
        if logical_commands(body)
            .iter()
            .any(|command| is_label_mutation(command) || is_final_body_mutation(command))
        {
            return Err(format!("metadata mutation is disconnected in function {name}"));
        }
    }
    Ok(())
}

fn validate_transaction(shell: &ShellStep<'_>) -> Result<(), String> {
    let commands = logical_commands(&shell.top_level);
    let refreshes = command_positions(&commands, |command| command == "refresh_version_pr_snapshot");
    let publishes = command_positions(&commands, |command| {
        command == "publish_version_pr_metadata \"$publication_phase\""
    });
    if refreshes.len() != 2 || publishes.len() != 2 {
        return Err(format!(
            "expected two snapshot refreshes and publications, found {}/{}",
            refreshes.len(),
            publishes.len()
        ));
    }
    let observed_identity = command_position(&commands, |command| {
        command.starts_with("gh pr view \"$pr_number\" ")
            && command.contains("$state_dir/observed-pr.json")
    })?;
    let observed_identity_argument = command_position(&commands, |command| {
        command
            == "observed_pr_args=(--observed-pr-json \"$state_dir/observed-pr.json\")"
    })?;
    let identity_authorization = command_position(&commands, |command| {
        command.starts_with("action=$(scripts/plan-version-pr-reconciliation ")
            && command.contains("--issue-json \"$state_dir/issue.json\"")
            && command.contains("${observed_pr_args[@]}")
    })?;
    let provisional_render = command_position(&commands, |command| {
        command == "render_version_pr_metadata \"$publication_phase\""
    })?;
    let final_snapshot = refreshes[1];
    let positions = [
        ("provisional planner", command_position(&commands, |command| {
            command.starts_with("publication_phase=$(scripts/plan-version-pr-reconciliation ")
                && command.contains("--merge-message-checked false)")
        })?),
        ("first snapshot", refreshes[0]),
        ("observed identity", observed_identity),
        ("observed identity argument", observed_identity_argument),
        ("identity authorization", identity_authorization),
        ("provisional render", provisional_render),
        ("final snapshot", final_snapshot),
        ("provisional publication", publishes[0]),
        ("rebuilt PR state", command_position(&commands, |command| {
            command.starts_with("scripts/build-version-pr-state ")
        })?),
        ("label gate", command_position(&commands, |command| {
            command.starts_with("plugins/codexy/hooks/codexy-pr-label-check.sh ")
        })?),
        ("completion gate", command_position(&commands, |command| {
            command.starts_with("scripts/validate-plugin-config --check-completion-handoff ")
        })?),
        ("merge-message gate", command_position(&commands, |command| {
            command.starts_with("plugins/codexy/hooks/codexy-merge-message-check.sh ")
        })?),
        ("proven planner", command_position(&commands, |command| {
            command.starts_with("publication_phase=$(scripts/plan-version-pr-reconciliation ")
                && command.contains("--merge-message-checked true)")
        })?),
        ("proven publication", publishes[1]),
    ];
    ordered(&positions)?;
    if commands.iter().any(|command| is_label_mutation(command)) {
        return Err("label mutation is disconnected from publisher".into());
    }
    if commands
        .iter()
        .enumerate()
        .filter(|(_, command)| is_body_mutation(command))
        .any(|(index, _)| index <= provisional_render || index >= final_snapshot)
    {
        return Err("body mutation is outside the provisional publication boundary".into());
    }
    Ok(())
}

fn is_label_mutation(command: &str) -> bool {
    command.contains("repos/$GITHUB_REPOSITORY/issues/$pr_number/labels")
}

fn is_body_mutation(command: &str) -> bool {
    (command.contains("gh pr create ") || command.starts_with("gh pr edit "))
        && command.contains("--body-file \"$state_dir/metadata/body.md\"")
}

fn is_final_body_mutation(command: &str) -> bool {
    command.starts_with("gh pr edit ")
        && command.contains("--body-file \"$state_dir/metadata/body.md\"")
}

fn logical_commands(lines: &[&str]) -> Vec<String> {
    let mut commands = Vec::new();
    let mut command = String::new();
    for line in lines.iter().map(|line| line.trim()).filter(|line| !line.is_empty()) {
        if !command.is_empty() {
            command.push(' ');
        }
        if let Some(continued) = line.strip_suffix('\\') {
            command.push_str(continued.trim_end());
        } else {
            command.push_str(line);
            commands.push(std::mem::take(&mut command));
        }
    }
    if !command.is_empty() {
        commands.push(command);
    }
    commands
}

fn command_position(
    commands: &[String],
    predicate: impl Fn(&str) -> bool,
) -> Result<usize, String> {
    commands
        .iter()
        .position(|command| predicate(command))
        .ok_or_else(|| "required scoped command missing".into())
}

fn command_positions(commands: &[String], predicate: impl Fn(&str) -> bool) -> Vec<usize> {
    commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| predicate(command).then_some(index))
        .collect()
}

fn ordered(positions: &[(&str, usize)]) -> Result<(), String> {
    positions.windows(2).try_for_each(|pair| {
        (pair[0].1 < pair[1].1)
            .then_some(())
            .ok_or_else(|| format!("{} must precede {}", pair[0].0, pair[1].0))
    })
}
