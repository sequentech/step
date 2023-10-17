use uuid::Uuid;

pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug)]
pub enum Error {
    IncorrectPath,
    IDNotFound,
    ElectionConfigNotFound(Uuid),
    ContestConfigNotFound(Uuid),
    FSError(std::io::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        Self::FSError(val)
    }
}

impl std::error::Error for Error {}
