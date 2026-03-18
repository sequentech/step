// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    #[doc = "Error type used throughout the project for unified error handling. Wraps common error types and provides conversion from anyhow, string, file access, and integer conversion errors."]
    #[derive(Debug)]
    pub enum Error {
        /// Wrapper for `anyhow::Error`.
        Anyhow(err: anyhow::Error) {
            from()
        }
        /// Wrapper for string-based errors.
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        /// Error accessing a file, includes path and IO error.
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
        /// Error converting integer types.
        TryFromIntError(err: std::num::TryFromIntError) {
            from()
        }
    }
}

/// Unified result type for sequent-core, using the custom `Error` type by default.
pub type Result<T, E = Error> = std::result::Result<T, E>;
