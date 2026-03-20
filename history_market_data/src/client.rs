use chrono::NaiveDate;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::DbConfig;
use crate::error::Result;
use crate::models::{BondHistoryData, BondInfo};

/// Клиент для работы с базой данных исторических данных.
///
/// ## Потокобезопасность
///
/// `MarketDataClient: Send + Sync + Clone`.
/// Внутренний `PgPool` построен на `Arc<Pool<Postgres>>`, поэтому клиент
/// безопасно разделять между потоками и задачами tokio.
/// `Clone` — дешёвый: создаётся дополнительная ссылка на тот же пул.
///
/// ```no_run
/// use std::sync::Arc;
/// use history_market_data::MarketDataClient;
///
/// # async fn example() -> history_market_data::Result<()> {
/// let client = Arc::new(MarketDataClient::from_env().await?);
/// let client2 = Arc::clone(&client); // безопасно в нескольких задачах
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MarketDataClient {
    pool: PgPool,
}

impl MarketDataClient {
    /// Создать клиент из готового пула соединений.
    ///
    /// Используйте этот конструктор для внедрения зависимостей (DI)
    /// и написания тестов с моковым пулом.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создать клиент из явной конфигурации.
    ///
    /// Рекомендуемый способ в production: конфиг можно собрать из любого
    /// источника (env, конфиг-файл, secret manager) независимо от клиента.
    ///
    /// ```no_run
    /// use history_market_data::{DbConfig, MarketDataClient};
    ///
    /// # async fn example() -> history_market_data::Result<()> {
    /// let config = DbConfig::from_env()?;
    /// let client = MarketDataClient::with_config(&config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_config(config: &DbConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url())
            .await?;
        Ok(Self { pool })
    }

    /// Удобный shortcut: создать клиент напрямую из `.env` файла.
    ///
    /// Эквивалентно `DbConfig::from_env()` + `with_config`.
    ///
    /// ```no_run
    /// # use history_market_data::MarketDataClient;
    /// # async fn example() -> history_market_data::Result<()> {
    /// let client = MarketDataClient::from_env().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_env() -> Result<Self> {
        let config = DbConfig::from_env()?;
        Self::with_config(&config).await
    }

    /// Получить все свечи (исторические данные) для заданной даты.
    pub async fn get_candles_by_date(&self, date: NaiveDate) -> Result<Vec<BondHistoryData>> {
        let candles = sqlx::query_as::<_, BondHistoryData>("SELECT * FROM bond_bondhistorydata WHERE date = $1")
            .bind(date)
            .fetch_all(&self.pool)
            .await?;
        Ok(candles)
    }

    /// Получить исторические данные для конкретной облигации за дату.
    pub async fn get_bond_candle(&self, bond_id: i64, date: NaiveDate) -> Result<Option<BondHistoryData>> {
        let candle =
            sqlx::query_as::<_, BondHistoryData>("SELECT * FROM bond_bondhistorydata WHERE bond_id = $1 AND date = $2")
                .bind(bond_id)
                .bind(date)
                .fetch_optional(&self.pool)
                .await?;
        Ok(candle)
    }

    /// Получить исторические данные для облигации за диапазон дат.
    pub async fn get_bond_candles_range(
        &self,
        bond_id: i64,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<BondHistoryData>> {
        let candles = sqlx::query_as::<_, BondHistoryData>(
            "SELECT * FROM bond_bondhistorydata \
             WHERE bond_id = $1 AND date >= $2 AND date <= $3 \
             ORDER BY date ASC",
        )
        .bind(bond_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;
        Ok(candles)
    }

    /// Получить информацию об облигации по ID.
    pub async fn get_bond_info(&self, bond_id: i64) -> Result<Option<BondInfo>> {
        let bond = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE id = $1")
            .bind(bond_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(bond)
    }

    /// Получить информацию об облигации по ISIN коду.
    pub async fn get_bond_by_isin(&self, isin: &str) -> Result<Option<BondInfo>> {
        let bond = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE isin = $1")
            .bind(isin)
            .fetch_optional(&self.pool)
            .await?;
        Ok(bond)
    }

    /// Получить список облигаций с пагинацией.
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

    /// Получить только торгуемые облигации.
    pub async fn get_traded_bonds(&self) -> Result<Vec<BondInfo>> {
        let bonds = sqlx::query_as::<_, BondInfo>("SELECT * FROM bond_bond WHERE is_traded = true")
            .fetch_all(&self.pool)
            .await?;
        Ok(bonds)
    }
}
