use std::error;
use std::fmt;

#[derive(Clone, Debug)]
pub(crate) enum Error {
    InvalidUrl,
    OperationTimedOut,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidUrl => "invalid url",
            Self::OperationTimedOut => "operation timed out",
        })
    }
}

impl error::Error for Error {}
