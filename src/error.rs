use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    DivisionByZero,
    Domain,
    NonFinite,
    Overflow,
    Underflow,
    Unsupported,
    Backend,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: &'static str,
}

impl Error {
    pub const fn new(kind: ErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Error {}
