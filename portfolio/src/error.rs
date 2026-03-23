use thiserror::Error;

/// Типизированные ошибки модуля `portfolio`.
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

    /// Портфель не найден.
    #[error("портфель с id={0} не найден")]
    PortfolioNotFound(i64),
}

pub type Result<T> = std::result::Result<T, Error>;
