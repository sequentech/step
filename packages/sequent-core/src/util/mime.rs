// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Returns all MIME types for the given file extension.
/// Falls back to `["application/json"]` if none is found.
pub fn get_mime_types(extension: &str) -> &'static [&'static str] {
    // Static array of (extension, list-of-mime-types) tuples.
    static MIME_TYPES: [(&str, &[&str]); 27] = [
        ("jpg",  &["image/jpeg"]),
        ("jpeg", &["image/jpeg"]),
        ("png",  &["image/png"]),
        ("gif",  &["image/gif"]),
        ("bmp",  &["image/bmp"]),
        ("webp", &["image/webp"]),
        ("svg",  &["image/svg+xml"]),
        ("txt",  &["text/plain"]),
        ("html", &["text/html"]),
        ("htm",  &["text/html"]),
        ("csv",  &["text/csv"]),
        ("json", &["application/json"]),
        ("pdf",  &["application/pdf"]),
        ("doc",  &["application/msword"]),
        (
            "docx",
            &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        ),
        ("xls",  &["application/vnd.ms-excel"]),
        (
            "xlsx",
            &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        ),
        ("ppt",  &["application/vnd.ms-powerpoint"]),
        (
            "pptx",
            &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
        ),
        ("mp3",  &["audio/mpeg"]),
        ("wav",  &["audio/wav"]),
        ("mp4",  &["video/mp4"]),
        ("avi",  &["video/x-msvideo"]),
        ("mov",  &["video/quicktime"]),
        // For ZIP, multiple known MIME types:
        ("zip",  &["application/zip", "application/x-zip-compressed", "multipart/x-zip"]),
        ("tar",  &["application/x-tar"]),
        ("gz",   &["application/gzip"]),
    ];

    // Simple linear lookup through our static array
    for (ext, mimes) in MIME_TYPES.iter() {
        if *ext == extension {
            return mimes;
        }
    }

    // Fallback MIME if none found
    &["application/json"]
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
