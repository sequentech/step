// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    /// General-purpose error type for operations.
    #[derive(Debug)]
    pub enum Error {
        /// Wrapper around an [`anyhow::Error`].
        Anyhow(err: anyhow::Error) {
            from()
        }
        /// Simple string error message.
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        /// Failure reading or writing a file on disk.
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
        /// Integer conversion overflow or truncation.
        TryFromIntError(err: std::num::TryFromIntError) {
            from()
        }
    }
}

/// Convenience alias defaulting the error type to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
