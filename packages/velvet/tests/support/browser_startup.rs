// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Keep a browser-startup control beside the PDF tests. A DevTools connection
//! can disappear before the Rust client sees Chrome's actual failure message.

use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// Chrome in --single-process mode occasionally crashes at startup (SIGTRAP on
// hosted runners). Production PDF printing retries transient browser exits
// with a new browser, so one crash is not a failure; repeated ones are.
const ATTEMPTS: usize = 3;

#[test]
fn chrome_can_render_a_blank_page_with_the_pdf_runtime_flags() {
    let mut failures = Vec::new();
    for attempt in 1..=ATTEMPTS {
        match render_blank_page() {
            Ok(()) => return,
            Err(failure) => failures.push(format!("attempt {attempt}: {failure}")),
        }
        thread::sleep(Duration::from_secs(1));
    }
    panic!(
        "Chrome could not render a blank page with the PDF runtime flags:\n{}",
        failures.join("\n")
    );
}

fn render_blank_page() -> Result<(), String> {
    let directory = tempfile::tempdir().unwrap();
    let stdout = directory.path().join("page.html");
    let stderr = directory.path().join("chrome.log");
    let browser = headless_chrome::browser::default_executable()
        .expect("install Chrome/Chromium or configure CHROME before running PDF tests");
    let mut child = Command::new(browser)
        .args([
            "--headless",
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--disable-dev-shm-usage",
            "--single-process",
            "--no-zygote",
            "--enable-logging=stderr",
            "--dump-dom",
            "about:blank",
        ])
        .arg(format!(
            "--user-data-dir={}",
            directory.path().join("profile").display()
        ))
        .stdin(Stdio::null())
        .stdout(File::create(&stdout).unwrap())
        .stderr(File::create(&stderr).unwrap())
        .spawn()
        .expect("install Chrome before running the real PDF suite");
    let log = || String::from_utf8_lossy(&fs::read(&stderr).unwrap()).into_owned();

    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("Chrome startup timed out: {}", log()));
        }
        thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        return Err(format!("Chrome exited with {status}: {}", log()));
    }
    let page = fs::read_to_string(stdout).unwrap();
    if !page.contains("<html>") {
        return Err(format!("Chrome printed no page: {page:?}"));
    }
    Ok(())
}
