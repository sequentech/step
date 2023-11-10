use std::error::Error as StdError;

pub type Result<T, E = Box<dyn StdError>> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    TallyTypeNotFound,
    TallyTypeNotImplemented(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
