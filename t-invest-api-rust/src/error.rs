use thiserror::Error;

/// Алиас `Result`, в котором вариант `Err` — это `t_invest_api_rust::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// Ошибки при создании клиента
#[derive(Error, Debug)]
pub enum Error {
    /// Ошибка транспорта
    #[error("transport error")]
    Transport(#[from] tonic::transport::Error),

    /// Токен авторизации содержит недопустимые символы
    #[error("authorization token contains invalid characters")]
    InvalidAuthorizationTokenCharacters(#[from] tonic::metadata::errors::InvalidMetadataValue),
}
