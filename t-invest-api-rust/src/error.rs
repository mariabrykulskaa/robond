use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("transport error")]
    Transport(#[from] tonic::transport::Error),

    #[error("authorization token contains invalid characters")]
    InvalidAuthorizationTokenCharacters(#[from] tonic::metadata::errors::InvalidMetadataValue),
}
