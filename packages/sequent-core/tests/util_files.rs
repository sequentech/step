// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The file helpers reports and uploads use: an extension swap, and a named
//! temporary file that holds exactly the bytes written and disappears with
//! its handle.

#![cfg(all(feature = "default_features", feature = "reports"))]

use sequent_core::temp_path;
use sequent_core::util::path::change_file_extension;
use sequent_core::util::temp_path as util_temp_path;

#[test]
fn an_extension_is_replaced_or_added() {
    assert_eq!(
        change_file_extension("report.html", "pdf").as_deref(),
        Some("report.pdf")
    );
    assert_eq!(
        change_file_extension("dir/report", "pdf").as_deref(),
        Some("dir/report.pdf")
    );
    assert_eq!(
        change_file_extension("a.tar.gz", "zip").as_deref(),
        Some("a.tar.zip")
    );
}

fn holds_until_dropped<H, E: std::fmt::Debug>(
    written: Result<(H, String, u64), E>,
    data: &[u8],
) {
    let (handle, path, size) = written.unwrap();
    assert_eq!(size, data.len() as u64);
    assert_eq!(std::fs::read(&path).unwrap(), data);
    let name = std::path::Path::new(&path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(
        name.starts_with("demo-") && name.ends_with(".bin"),
        "{name}"
    );
    drop(handle);
    assert!(!std::path::Path::new(&path).exists());
}

#[test]
fn a_named_temp_file_holds_the_bytes_until_it_is_dropped() {
    let data = b"DEMO report bytes".to_vec();
    holds_until_dropped(
        temp_path::write_into_named_temp_file(&data, "demo-", ".bin"),
        &data,
    );
    holds_until_dropped(
        util_temp_path::write_into_named_temp_file(&data, "demo-", ".bin"),
        &data,
    );
}
