// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use lazy_static::lazy_static;
use std::collections::HashMap;

/// Static map of file extensions to a list of possible MIME types.
lazy_static! {
    static ref MIME_TYPES: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("jpg",  vec!["image/jpeg"]);
        m.insert("jpeg", vec!["image/jpeg"]);
        m.insert("png",  vec!["image/png"]);
        m.insert("gif",  vec!["image/gif"]);
        m.insert("bmp",  vec!["image/bmp"]);
        m.insert("webp", vec!["image/webp"]);
        m.insert("svg",  vec!["image/svg+xml"]);
        m.insert("txt",  vec!["text/plain"]);
        m.insert("html", vec!["text/html"]);
        m.insert("htm",  vec!["text/html"]);
        m.insert("csv",  vec!["text/csv"]);
        m.insert("json", vec!["application/json"]);
        m.insert("pdf",  vec!["application/pdf"]);
        m.insert("doc",  vec!["application/msword"]);
        m.insert("docx", vec!["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]);
        m.insert("xls",  vec!["application/vnd.ms-excel"]);
        m.insert("xlsx", vec!["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
        m.insert("ppt",  vec!["application/vnd.ms-powerpoint"]);
        m.insert("pptx", vec!["application/vnd.openxmlformats-officedocument.presentationml.presentation"]);
        m.insert("mp3",  vec!["audio/mpeg"]);
        m.insert("wav",  vec!["audio/wav"]);
        m.insert("mp4",  vec!["video/mp4"]);
        m.insert("avi",  vec!["video/x-msvideo"]);
        m.insert("mov",  vec!["video/quicktime"]);
        // For ZIP, include multiple known MIME types:
        m.insert("zip",  vec!["application/zip", "application/x-zip-compressed", "multipart/x-zip"]);
        m.insert("tar",  vec!["application/x-tar"]);
        m.insert("gz",   vec!["application/gzip"]);
        m
    };
}

/// Returns all MIME types for the given file extension.
/// Falls back to `["application/json"]` if none is found.
pub fn get_mime_types(extension: &str) -> &'static [&'static str] {
    MIME_TYPES
        .get(extension)
        .map(|mimes| &mimes[..]) // Return a slice of &str
        .unwrap_or(&["application/json"])
}

/// Checks if a given extension is associated with the specified MIME type.
pub fn matches_mime(extension: &str, mime_type: &str) -> bool {
    get_mime_types(extension).contains(&mime_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_mime_types() {
        let zip_mimes = get_mime_types("zip");
        assert_eq!(zip_mimes.len(), 3);
        assert!(zip_mimes.contains(&"application/zip"));
        assert!(zip_mimes.contains(&"application/x-zip-compressed"));
        assert!(zip_mimes.contains(&"multipart/x-zip"));
    }

    #[test]
    fn test_matches_mime() {
        assert!(matches_mime("zip", "application/zip"));
        assert!(!matches_mime("zip", "application/pdf"));
    }

    #[test]
    fn test_unknown_extension() {
        // Unknown extension should return the default "application/json"
        let mimes = get_mime_types("unknownext");
        assert_eq!(mimes, ["application/json"]);
    }
}
