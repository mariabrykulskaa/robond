use thiserror::Error;

/// Типизированные ошибки модуля `history_market_data`.
#[derive(Debug, Error)]
pub enum Error {
    /// Ошибка на уровне базы данных (sqlx).
    #[error("ошибка базы данных: {0}")]
    Database(#[from] sqlx::Error),

    /// Отсутствует обязательная переменная окружения.
    #[error("отсутствует переменная окружения `{0}`")]
    MissingEnvVar(String),

    /// Переменная окружения содержит некорректное значение.
    #[error("некорректное значение переменной `{var}`: {source}")]
    InvalidEnvVar {
        var: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Псевдоним результата для всех публичных методов этого модуля.
pub type Result<T> = std::result::Result<T, Error>;
