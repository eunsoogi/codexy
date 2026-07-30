use std::{fs, io, path::Path};

/// Sources a POSIX fixture script after binding bare command names to mock payloads.
///
/// Git Bash can reject an extensionless PATH script despite a valid shebang and
/// chmod state. A shell function keeps the production script's bare invocation
/// while directly interpreting the projected fixture payload through `sh`.
pub(crate) fn write_posix_fixture_shell_runner(
    path: &Path,
    target_environment: &str,
    bindings: &[(&str, &str)],
) -> io::Result<()> {
    let mut source = String::from("#!/bin/sh\nset -eu\n");
    for (command, payload_environment) in bindings {
        validate_identifier(command)?;
        validate_identifier(payload_environment)?;
        source.push_str(&format!(
            "{command}() {{ sh \"${payload_environment}\" \"$@\"; }}\n"
        ));
    }
    validate_identifier(target_environment)?;
    source.push_str(&format!(". \"${target_environment}\" \"$@\"\n"));
    fs::write(path, source)?;
    super::make_executable(path)
}

pub(crate) fn write_single_posix_fixture_shell_runner(
    path: &Path,
    target_environment: &str,
    command: &str,
    payload_environment: &str,
) -> io::Result<()> {
    write_posix_fixture_shell_runner(path, target_environment, &[(command, payload_environment)])
}

fn validate_identifier(value: &str) -> io::Result<()> {
    let mut characters = value.chars();
    let starts_identifier = matches!(characters.next(), Some('_' | 'a'..='z' | 'A'..='Z'));
    let continues_identifier =
        characters.all(|character| character == '_' || character.is_ascii_alphanumeric());
    let reserved = matches!(
        value,
        "case"
            | "do"
            | "done"
            | "elif"
            | "else"
            | "esac"
            | "fi"
            | "for"
            | "if"
            | "in"
            | "then"
            | "until"
            | "while"
    );
    (starts_identifier && continues_identifier && !reserved)
        .then_some(())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture shell identifier"))
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use super::write_posix_fixture_shell_runner;
    use crate::support::{FixtureCommand, write_posix_fixture_command};

    #[test]
    fn shell_runner_rejects_unsafe_function_identifiers_before_writing()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        for identifier in ["", "9git", "git-name", "if"] {
            let runner = temp.path().join(format!("{identifier}.sh"));
            let error = write_posix_fixture_shell_runner(
                &runner,
                "CODEXY_FIXTURE_TARGET",
                &[(identifier, "CODEXY_FIXTURE_GIT")],
            )
            .expect_err("unsafe shell function identifier must fail closed");
            assert_eq!(error.kind(), ErrorKind::InvalidInput, "{identifier:?}");
            assert!(!runner.exists(), "{identifier:?} wrote a runner");
        }
        Ok(())
    }

    #[test]
    fn shell_runner_executes_the_safe_fixture_identifiers() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let target = temp.path().join("target.sh");
        write_posix_fixture_command(&target, "#!/bin/sh\ngit first\njq second\ngh third\n")?;
        let runner = temp.path().join("runner.sh");
        let bindings = [
            ("git", "CODEXY_FIXTURE_GIT"),
            ("jq", "CODEXY_FIXTURE_JQ"),
            ("gh", "CODEXY_FIXTURE_GH"),
        ];
        write_posix_fixture_shell_runner(&runner, "CODEXY_FIXTURE_TARGET", &bindings)?;
        for (name, _) in bindings {
            let payload = temp.path().join(name);
            write_posix_fixture_command(
                &payload,
                &format!("#!/bin/sh\nprintf '{name}:%s\\n' \"$1\"\n"),
            )?;
        }
        let output = FixtureCommand::new(&runner)
            .env_path("CODEXY_FIXTURE_TARGET", &target)
            .env_path("CODEXY_FIXTURE_GIT", temp.path().join("git"))
            .env_path("CODEXY_FIXTURE_JQ", temp.path().join("jq"))
            .env_path("CODEXY_FIXTURE_GH", temp.path().join("gh"))
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)?,
            "git:first\njq:second\ngh:third\n"
        );
        Ok(())
    }
}
