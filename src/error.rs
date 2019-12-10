use std::{
    error::Error as StdError,
    ffi::{IntoStringError, NulError},
    fmt,
    result::Result as StdResult,
    str::Utf8Error,
};

/// Describes all errors that may occur
#[derive(Debug)]
pub enum Error {
    /// Error when converting a CString into a String
    IntoString(IntoStringError),
    /// Interior nul byte was found
    NulByte(NulError),
    /// Got a NULL pointer
    Null,
    /// Can not create UTF-8 string
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

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            IntoString(ref err) => write!(out, "string conversion error: {}", err),
            NulByte(ref err) => write!(out, "nul byte error: {}", err),
            Null => write!(out, "got a NULL pointer"),
            Utf8(ref err) => write!(out, "UTF-8 error: {}", err),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        use self::Error::*;
        Some(match self {
            IntoString(ref err) => err,
            NulByte(ref err) => err,
            Utf8(ref err) => err,
            _ => return None,
        })
    }
}

/// A specialized result type for FFI utilities
pub type Result<T> = StdResult<T, Error>;
