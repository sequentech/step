// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing secrets stay out of the logs: shell commands carry passwords and
//! their output is data, and the ACM private key is neither logged nor shown
//! by `Debug`.

#![cfg(all(
    feature = "signatures",
    feature = "keycloak",
    feature = "default_features"
))]

#[path = "support/logs.rs"]
mod logs;

use logs::with_captured_logs;
use sequent_core::signatures::ecies_encrypt::{ecies_sign_data, EciesKeyPair};
use sequent_core::signatures::shell::run_shell_command;
use std::future::Future;

fn captured<T>(run: impl Future<Output = T>) -> (T, String) {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime")
        .block_on(with_captured_logs(run))
}

const PASSWORD: &str = "ShellPasswordValue3a7c";
const OUTPUT: &str = "ShellOutputValue8e1d";
const PRIVATE_KEY: &str = "PrivateKeyPemValue5b9e";
const PUBLIC_KEY: &str = "PublicKeyPemValue2d6a";

fn key_pair() -> EciesKeyPair {
    EciesKeyPair {
        private_key_pem: PRIVATE_KEY.to_string(),
        public_key_pem: PUBLIC_KEY.to_string(),
    }
}

#[test]
fn shell_commands_and_their_output_stay_out_of_the_logs() {
    let ((output, failure), logs) = captured(async {
        (
            run_shell_command(&format!(
                "printf {OUTPUT} # -passin pass:{PASSWORD}"
            )),
            run_shell_command(&format!(
                "printf {OUTPUT}; printf {PASSWORD} >&2; exit 1 # -passin pass:{PASSWORD}"
            )),
        )
    });

    assert_eq!(output.expect("the command runs"), OUTPUT);
    let failure = format!("{:?}", failure.expect_err("the command fails"));
    assert!(logs.contains("run_shell_command"), "{logs}");
    for value in [PASSWORD, OUTPUT] {
        assert!(!logs.contains(value), "{value} in logs: {logs}");
        assert!(!failure.contains(value), "{value} in error: {failure}");
    }
}

#[test]
fn key_pair_debug_shows_only_the_public_key() {
    let debug = format!("{:?}", key_pair());

    assert!(debug.contains(PUBLIC_KEY), "{debug}");
    assert!(!debug.contains(PRIVATE_KEY), "{debug}");
}

#[test]
fn signing_logs_no_private_key() {
    // The key isn't a valid PEM (and the ECIES tool may be missing), so signing
    // fails after the key has been handled.
    let (_, logs) = captured(async { ecies_sign_data(&key_pair(), "data") });

    assert!(logs.contains("ecies_sign_data"), "{logs}");
    assert!(!logs.contains(PRIVATE_KEY), "private key in logs: {logs}");
}
