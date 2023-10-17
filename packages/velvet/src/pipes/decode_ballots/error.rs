pub type Result<T, E = Error> = std::result::Result<T, E>;
pub use crate::pipes::error::Error as PipeError;

#[derive(Debug)]
pub enum Error {
    ConfigNotValid,
    PipeError(PipeError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<PipeError> for Error {
    fn from(val: PipeError) -> Self {
        Self::PipeError(val)
    }
}

impl std::error::Error for Error {}
