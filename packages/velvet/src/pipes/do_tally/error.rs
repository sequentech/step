pub use crate::pipes::error::Error as PipesError;
use std::error::Error as StdError;

pub type Result<T, E = Box<dyn StdError>> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    JsonParse(serde_json::Error),
    FromPipes(PipesError),
    FS(std::io::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::JsonParse(val)
    }
}

impl From<PipesError> for Error {
    fn from(val: PipesError) -> Self {
        Self::FromPipes(val)
    }
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        Self::FS(val)
    }
}

impl std::error::Error for Error {}
