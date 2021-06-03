use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("incomplete utf8 sequence `{0:?}`")]
    IncompleteUtf8Sequence(Vec<u8>),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    IOError(#[from] futures_io::Error),
}
