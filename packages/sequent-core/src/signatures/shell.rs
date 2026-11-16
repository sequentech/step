// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use std::process::Command;
use tracing::{info, instrument};

/// Runs `command` with `sh -c`. Callers pass passwords and key paths in the
/// command, so neither the command nor its output is recorded in the logs.
#[instrument(skip(command), err)]
pub fn run_shell_command(command: &str) -> Result<String> {
    // Run the shell command
    let output = Command::new("sh").arg("-c").arg(command).output()?;

    // Check if the command was successful
    if !output.status.success() {
        // The exit status only: stdout and stderr may echo the command's
        // secrets, and the error is logged.
        return Err(anyhow::anyhow!("Shell command failed: {}", output.status));
    }

    // Convert the output to a string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Return the output
    Ok(stdout.to_string())
}

#[cfg(all(test, any(feature = "tracing", feature = "reports")))]
mod tests {
    use super::run_shell_command;
    use crate::util::log_capture::LogCapture;

    const PASSWORD: &str = "ShellPasswordValue3a7c";
    const OUTPUT: &str = "ShellOutputValue8e1d";

    #[test]
    fn commands_and_their_output_stay_out_of_the_logs() {
        let (capture, _guard) = LogCapture::install();

        let output = run_shell_command(&format!(
            "printf {OUTPUT} # -passin pass:{PASSWORD}"
        ))
        .expect("the command runs");
        let failure = run_shell_command(&format!(
            "printf {OUTPUT}; printf {PASSWORD} >&2; exit 1 # -passin pass:{PASSWORD}"
        ))
        .expect_err("the command fails");

        assert_eq!(output, OUTPUT);
        let failure = format!("{failure:?}");
        let logs = capture.contents();
        assert!(logs.contains("run_shell_command"), "{logs}");
        for value in [PASSWORD, OUTPUT] {
            assert!(!logs.contains(value), "{value} in logs: {logs}");
            assert!(!failure.contains(value), "{value} in error: {failure}");
        }
    }
}
