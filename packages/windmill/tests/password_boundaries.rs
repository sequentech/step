// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Transmission-package encryption uses this generator. Assert its observable
//! contract without statistical tests that occasionally fail by chance.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use windmill::services::password::generate_random_string_with_charset;

#[test]
fn generated_passwords_have_the_requested_length_and_only_approved_characters() {
    for (length, alphabet) in [(1, "x"), (64, "0123456789abcdef"), (17, "éλ中")] {
        let password = generate_random_string_with_charset(length, alphabet);
        assert_eq!(password.chars().count(), length);
        assert!(password
            .chars()
            .all(|character| alphabet.contains(character)));
    }
    assert_eq!(generate_random_string_with_charset(0, ""), "");
}

#[derive(Clone)]
struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedLogs {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn generated_encryption_passwords_never_enter_application_logs() {
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let writer = CapturedLogs(bytes.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .without_time()
        .with_writer(move || writer.clone())
        .finish();

    let password = tracing::subscriber::with_default(subscriber, || {
        // A positive control proves that an empty/misconfigured capture cannot
        // make the absence-of-secrets assertion pass vacuously.
        tracing::info!("password-log-capture-control");
        generate_random_string_with_charset(64, "0123456789abcdef")
    });
    let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("password-log-capture-control"));
    assert!(
        !logs.contains(&password),
        "generated password leaked into tracing output"
    );
}
