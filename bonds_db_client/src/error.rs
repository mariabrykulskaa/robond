use thiserror::Error;

/// Алиас `Result`, в котором вариант `Err` — это `bonds_db_client::Error`.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),

    #[error("decode error")]
    Decode(#[from] prost::DecodeError),

    #[error("uuid error")]
    Uuid(#[from] uuid::Error),
}
