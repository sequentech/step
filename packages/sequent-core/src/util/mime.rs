// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

pub fn get_mime_type(extension: &str) -> &str {
    let mime_types: HashMap<&str, &str> = [
        ("jpg", "image/jpeg"),
        ("jpeg", "image/jpeg"),
        ("png", "image/png"),
        ("gif", "image/gif"),
        ("bmp", "image/bmp"),
        ("webp", "image/webp"),
        ("svg", "image/svg+xml"),
        ("txt", "text/plain"),
        ("html", "text/html"),
        ("htm", "text/html"),
        ("csv", "text/csv"),
        ("json", "application/json"),
        ("pdf", "application/pdf"),
        ("doc", "application/msword"),
        (
            "docx",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        ("xls", "application/vnd.ms-excel"),
        (
            "xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
        ("ppt", "application/vnd.ms-powerpoint"),
        (
            "pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        ("mp3", "audio/mpeg"),
        ("wav", "audio/wav"),
        ("mp4", "video/mp4"),
        ("avi", "video/x-msvideo"),
        ("mov", "video/quicktime"),
        ("zip", "application/zip"),
        ("tar", "application/x-tar"),
        ("gz", "application/gzip"),
    ]
    .iter()
    .cloned()
    .collect();

    mime_types.get(extension).unwrap_or(&"application/json")
}
