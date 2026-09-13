// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Keep a browser-startup control beside the PDF tests. A DevTools connection
//! can disappear before the Rust client sees Chrome's actual failure message.

use std::fs::{self, File};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn chrome_can_render_a_blank_page_with_the_pdf_runtime_flags() {
    let directory = tempfile::tempdir().unwrap();
    let stdout = directory.path().join("page.html");
    let stderr = directory.path().join("chrome.log");
    let mut child = Command::new("google-chrome-stable")
        .args([
            "--headless",
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--disable-dev-shm-usage",
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

    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "Chrome startup timed out: {}",
                fs::read_to_string(&stderr).unwrap()
            );
        }
        thread::sleep(Duration::from_millis(50));
    };
    assert!(
        status.success(),
        "Chrome exited with {status}: {}",
        fs::read_to_string(&stderr).unwrap()
    );
    assert!(fs::read_to_string(stdout).unwrap().contains("<html>"));
}
