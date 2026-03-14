use crate::error::{Error, Result};

/// Конфигурация подключения к базе данных.
///
/// Разделяет ответственность конфигурирования от бизнес-логики клиента.
/// Позволяет создавать [`crate::MarketDataClient`] из любого источника данных:
/// переменных окружения, конфиг-файла, секрет-менеджера и т.д.
///
/// # Пример
///
/// ```no_run
/// use history_market_data::DbConfig;
///
/// let config = DbConfig::from_env()?;
/// println!("Подключение к {}:{}", config.host, config.port);
/// # Ok::<(), history_market_data::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    /// Максимальное число соединений в пуле (по умолчанию 5).
    pub max_connections: u32,
}

impl DbConfig {
    /// Создать конфигурацию из переменных окружения (`.env` файл).
    ///
    /// Читает: `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USERNAME`, `DB_PASSWORD`.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            host: env_var("DB_HOST")?,
            port: env_var("DB_PORT")?.parse::<u16>().map_err(|e| Error::InvalidEnvVar {
                var: "DB_PORT",
                source: Box::new(e),
            })?,
            database: env_var("DB_NAME")?,
            username: env_var("DB_USERNAME")?,
            password: env_var("DB_PASSWORD")?,
            max_connections: 5,
        })
    }

    pub(crate) fn database_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

fn env_var(name: &'static str) -> Result<String> {
    std::env::var(name).map_err(|_| Error::MissingEnvVar(name.to_string()))
}
