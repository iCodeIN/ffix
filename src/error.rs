use std::{
    ffi::{IntoStringError, NulError},
    result::Result as StdResult,
    str::Utf8Error,
};

// TODO: impl display and std error
#[derive(Debug)]
pub enum Error {
    IntoString(IntoStringError),
    NulByte(NulError),
    Null,
    Utf8(Utf8Error),
}

impl From<IntoStringError> for Error {
    fn from(err: IntoStringError) -> Self {
        Self::IntoString(err)
    }
}

impl From<NulError> for Error {
    fn from(err: NulError) -> Self {
        Self::NulByte(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}

pub type Result<T> = StdResult<T, Error>;
