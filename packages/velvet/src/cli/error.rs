use crate::pipes;
use std::error::Error as StdError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    ConfigNotFound,
    CannotOpenConfig,
    Json(serde_json::Error),
    StageDefinition(String),
    StageNotFound,
    PipeNotFound,
    FromPipe(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

// impl From<pipes::error::Error> for Error {
//     fn from(val: Box<dyn StdError>) -> Self {
//         Self::FromPipe(val)
//     }
// }

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::Json(val)
    }
}

impl std::error::Error for Error {}
