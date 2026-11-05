// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use std::ffi::OsStr;
use std::process::Command;
use tracing::instrument;

#[instrument(skip(command), err)]
pub fn run_shell_command(command: &str) -> Result<String> {
    // Run the shell command
    let output = Command::new("sh").arg("-c").arg(command).output()?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(anyhow!("Shell command failed with {}", output.status));
    }

    // Convert the output to a string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Return the output
    Ok(stdout.to_string())
}

/// Builds a command that runs `program` directly with `args`, without a
/// shell, so that no argument is ever interpreted by a shell.
pub fn build_command<S: AsRef<OsStr>>(program: &str, args: &[S]) -> Command {
    let mut command = Command::new(program);
    command.args(args);
    command
}

/// Runs a command and returns its stdout. Fails if the command does not
/// exit successfully. Neither the arguments nor stderr are logged or
/// included in the error, since a failing program may echo a secret
/// argument.
#[instrument(skip(command), err)]
pub fn run_command(mut command: Command) -> Result<String> {
    let program = command.get_program().to_string_lossy().to_string();
    let output = command
        .output()
        .with_context(|| format!("Failed to run {program}"))?;
    if !output.status.success() {
        return Err(anyhow!("{program} failed with {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_does_not_interpret_shell_syntax() {
        let argument = "a; echo injected $(echo x) `echo y`";
        let output =
            run_command(build_command("printf", &["%s", argument])).unwrap();
        assert_eq!(output, argument);
    }

    #[test]
    fn test_run_command_fails_on_non_zero_exit() {
        assert!(run_command(build_command::<&str>("false", &[])).is_err());
    }

    #[test]
    fn test_run_command_error_omits_stderr() {
        let error = run_command(build_command(
            "sh",
            &["-c", "echo \"$0\" >&2; exit 1", "secret-password"],
        ))
        .unwrap_err();
        assert!(!format!("{error:?}").contains("secret-password"));
    }

    #[test]
    fn test_run_shell_command_error_omits_command() {
        let error = run_shell_command("false secret-password").unwrap_err();
        assert!(!format!("{error:?}").contains("secret-password"));
    }
}
