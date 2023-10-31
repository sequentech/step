// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use uuid::Uuid;

pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug)]
pub enum Error {
    IncorrectPath,
    IDNotFound,
    ElectionConfigNotFound(Uuid),
    ContestConfigNotFound(Uuid),
    IO(std::path::PathBuf, std::io::Error),
    FromPipe(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        Self::FS(val)
    }
}

impl std::error::Error for Error {}
