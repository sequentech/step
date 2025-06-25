// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Anyhow(err: anyhow::Error) {
            from()
        }
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
        TryFromIntError(err: std::num::TryFromIntError) {
            from()
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
