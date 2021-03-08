use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    IOError(#[from] futures_io::Error),
}
