use anyhow::Result;
use chrono::NaiveDate;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::io::{self, Write};

use crate::models::{BondHistoryData, BondInfo};

/// Клиент для работы с базой данных исторических данных
pub struct MarketDataClient {
    pool: PgPool,
}

impl MarketDataClient {
    /// Создать новое подключение к базе данных
    ///
    /// # Аргументы
    /// * `database_url` - URL подключения к PostgreSQL в формате:
    ///   `postgresql://username:password@host:port/database`
    ///   
    /// # Пример
    /// ```no_run
    /// # use history_market_data::MarketDataClient;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = MarketDataClient::new(
    ///     "postgresql://Maria:password@79.174.88.198:16305/HedgehogFinanceDB"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(database_url).await?;

        Ok(Self { pool })
    }

    /// Создать подключение из отдельных параметров
    ///
    /// # Аргументы
    /// * `host` - Хост базы данных (например, "79.174.88.198")
    /// * `port` - Порт (например, 16305)
    /// * `database` - Имя базы данных (например, "HedgehogFinanceDB")
    /// * `username` - Имя пользователя
    /// * `password` - Пароль
    ///   
    /// # Пример
    /// ```no_run
    /// # use history_market_data::MarketDataClient;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = MarketDataClient::from_credentials(
    ///     "79.174.88.198",
    ///     16305,
    ///     "HedgehogFinanceDB",
    ///     "username",
    ///     "password"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_credentials(
        host: &str,
        port: u16,
        database: &str,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let database_url = format!("postgresql://{}:{}@{}:{}/{}", username, password, host, port, database);
        Self::new(&database_url).await
    }

    /// Создать подключение с интерактивным вводом учетных данных из консоли
    ///
    /// # Аргументы
    /// * `host` - Хост базы данных (например, "79.174.88.198")
    /// * `port` - Порт (например, 16305)
    /// * `database` - Имя базы данных (например, "HedgehogFinanceDB")
    ///   
    /// # Пример
    /// ```no_run
    /// # use history_market_data::MarketDataClient;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = MarketDataClient::connect_interactive(
    ///     "79.174.88.198",
    ///     16305,
    ///     "HedgehogFinanceDB"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_interactive(host: &str, port: u16, database: &str) -> Result<Self> {
        // Запрашиваем имя пользователя
        print!("Введите имя пользователя: ");
        io::stdout().flush()?;
        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        let username = username.trim();

        // Запрашиваем пароль (скрытый ввод)
        print!("Введите пароль: ");
        io::stdout().flush()?;
        let password = rpassword::read_password()?;

        Self::from_credentials(host, port, database, username, &password).await
    }

    /// Получить все свечи (исторические данные) для заданной даты
    ///
    /// # Аргументы
    /// * `date` - Дата, для которой нужно получить данные
    ///
    /// # Возвращает
    /// Вектор всех записей BondHistoryData за указанную дату
    pub async fn get_candles_by_date(&self, date: NaiveDate) -> Result<Vec<BondHistoryData>> {
        let candles = sqlx::query_as::<_, BondHistoryData>("SELECT * FROM bond_bondhistorydata WHERE date = $1")
            .bind(date)
            .fetch_all(&self.pool)
            .await?;

        Ok(candles)
    }

    /// Получить исторические данные для конкретной облигации за дату
    ///
    /// # Аргументы
    /// * `bond_id` - ID облигации
    /// * `date` - Дата
    pub async fn get_bond_candle(&self, bond_id: i64, date: NaiveDate) -> Result<Option<BondHistoryData>> {
        let candle =
            sqlx::query_as::<_, BondHistoryData>("SELECT * FROM bond_bondhistorydata WHERE bond_id = $1 AND date = $2")
                .bind(bond_id)
                .bind(date)
                .fetch_optional(&self.pool)
                .await?;

        Ok(candle)
    }

    /// Получить исторические данные для облигации за диапазон дат
    ///
    /// # Аргументы
    /// * `bond_id` - ID облигации
    /// * `start_date` - Начальная дата (включительно)
    /// * `end_date` - Конечная дата (включительно)
    pub async fn get_bond_candles_range(
        &self,
        bond_id: i64,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BondHistoryData>> {
        let candles = sqlx::query_as::<_, BondHistoryData>(
            "SELECT * FROM bond_bondhistorydata 
             WHERE bond_id = $1 AND date >= $2 AND date <= $3 
             ORDER BY date ASC",
        )
        .bind(bond_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(candles)
    }

    /// Получить информацию об облигации по ID
    ///
    /// # Аргументы
    /// * `bond_id` - ID облигации
    pub async fn get_bond_info(&self, bond_id: i64) -> Result<Option<BondInfo>> {
        let bond = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE id = $1")
            .bind(bond_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(bond)
    }

    /// Получить информацию об облигации по ISIN коду
    ///
    /// # Аргументы
    /// * `isin` - ISIN код облигации
    pub async fn get_bond_by_isin(&self, isin: &str) -> Result<Option<BondInfo>> {
        let bond = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE isin = $1")
            .bind(isin)
            .fetch_optional(&self.pool)
            .await?;

        Ok(bond)
    }

    /// Получить список всех облигаций
    ///
    /// # Аргументы
    /// * `limit` - Максимальное количество записей (опционально)
    /// * `offset` - Смещение для пагинации (опционально)
    pub async fn get_all_bonds(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<BondInfo>> {
        let limit = limit.unwrap_or(1000);
        let offset = offset.unwrap_or(0);

        let bonds = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond ORDER BY id LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(bonds)
    }

    /// Получить только торгуемые облигации
    pub async fn get_traded_bonds(&self) -> Result<Vec<BondInfo>> {
        let bonds = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE is_traded = true")
            .fetch_all(&self.pool)
            .await?;

        Ok(bonds)
    }
}
