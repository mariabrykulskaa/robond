use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::models::{Portfolio, PortfolioCash, PortfolioHolding, PortfolioSnapshot};

/// Клиент для работы с портфелями в PostgreSQL.
///
/// `PortfolioClient: Send + Sync + Clone`.
/// Внутренний `PgPool` построен на `Arc`, поэтому клиент безопасно
/// разделять между потоками и задачами tokio.
#[derive(Clone)]
pub struct PortfolioClient {
    pool: PgPool,
}

impl PortfolioClient {
    /// Создать клиент из готового пула соединений.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Создать клиент из конфигурации `history_market_data::DbConfig`.
    pub async fn with_config(config: &history_market_data::DbConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url())
            .await?;
        Ok(Self { pool })
    }

    /// Создать клиент из `.env` файла.
    pub async fn from_env() -> Result<Self> {
        let config = history_market_data::DbConfig::from_env().map_err(|e| Error::MissingEnvVar(e.to_string()))?;
        Self::with_config(&config).await
    }

    /// Применить миграции (создать таблицы, если их нет).
    pub async fn run_migrations(&self) -> Result<()> {
        let migrations = [
            include_str!("../migrations/001_create_portfolio_tables.sql"),
            include_str!("../migrations/002_create_users_table.sql"),
            include_str!("../migrations/003_add_user_id_to_portfolio.sql"),
            include_str!("../migrations/004_add_tinvest_and_strategy.sql"),
            include_str!("../migrations/005_move_tinvest_to_portfolio.sql"),
            include_str!("../migrations/006_add_pending_strategy_run.sql"),
        ];
        for sql in migrations {
            sqlx::raw_sql(sql).execute(&self.pool).await?;
        }
        Ok(())
    }

    // ── Портфель ───────────────────────────────────────────────

    /// Создать новый портфель.
    pub async fn create_portfolio(&self, name: &str) -> Result<Portfolio> {
        let row =
            sqlx::query_as::<_, Portfolio>("INSERT INTO portfolio (name) VALUES ($1) RETURNING id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    /// Получить портфель по id.
    pub async fn get_portfolio(&self, portfolio_id: i64) -> Result<Portfolio> {
        sqlx::query_as::<_, Portfolio>(
            "SELECT id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at FROM portfolio WHERE id = $1",
        )
        .bind(portfolio_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::PortfolioNotFound(portfolio_id))
    }

    /// Список всех портфелей.
    pub async fn list_portfolios(&self) -> Result<Vec<Portfolio>> {
        let rows = sqlx::query_as::<_, Portfolio>(
            "SELECT id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at FROM portfolio ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Методы с привязкой к пользователю ─────────────────────

    /// Создать портфель для конкретного пользователя.
    pub async fn create_portfolio_for_user(&self, user_id: i64, name: &str) -> Result<Portfolio> {
        let row = sqlx::query_as::<_, Portfolio>(
            "INSERT INTO portfolio (name, user_id) VALUES ($1, $2) RETURNING id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at",
        )
        .bind(name)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Список портфелей пользователя.
    pub async fn list_portfolios_for_user(&self, user_id: i64) -> Result<Vec<Portfolio>> {
        let rows = sqlx::query_as::<_, Portfolio>(
            "SELECT id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at FROM portfolio WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Получить портфель по id, проверяя принадлежность пользователю.
    pub async fn get_portfolio_for_user(&self, user_id: i64, portfolio_id: i64) -> Result<Portfolio> {
        sqlx::query_as::<_, Portfolio>(
            "SELECT id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at FROM portfolio WHERE id = $1 AND user_id = $2",
        )
        .bind(portfolio_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::PortfolioNotFound(portfolio_id))
    }

    // ── Стратегии ──────────────────────────────────────────────

    /// Назначить стратегию на портфель.
    pub async fn set_strategy(&self, portfolio_id: i64, strategy_name: &str) -> Result<Portfolio> {
        sqlx::query_as::<_, Portfolio>(
            "UPDATE portfolio SET strategy_name = $2, strategy_running = false
             WHERE id = $1
             RETURNING id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at",
        )
        .bind(portfolio_id)
        .bind(strategy_name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::PortfolioNotFound(portfolio_id))
    }

    /// Убрать стратегию с портфеля.
    pub async fn clear_strategy(&self, portfolio_id: i64) -> Result<Portfolio> {
        sqlx::query_as::<_, Portfolio>(
            "UPDATE portfolio SET strategy_name = NULL, strategy_running = false
             WHERE id = $1
             RETURNING id, name, user_id, strategy_name, strategy_running, pending_strategy_run, created_at",
        )
        .bind(portfolio_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::PortfolioNotFound(portfolio_id))
    }

    // ── Позиции (облигации) ────────────────────────────────────

    /// Установить количество облигаций ISIN в портфеле (upsert).
    pub async fn set_holding(&self, portfolio_id: i64, isin: &str, quantity: i64) -> Result<PortfolioHolding> {
        let row = sqlx::query_as::<_, PortfolioHolding>(
            "INSERT INTO portfolio_holding (portfolio_id, isin, quantity, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (portfolio_id, isin) DO UPDATE
                SET quantity = EXCLUDED.quantity, updated_at = now()
             RETURNING id, portfolio_id, isin, quantity, updated_at",
        )
        .bind(portfolio_id)
        .bind(isin)
        .bind(quantity)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Изменить количество облигаций на delta (+ покупка, – продажа).
    pub async fn adjust_holding(&self, portfolio_id: i64, isin: &str, delta: i64) -> Result<PortfolioHolding> {
        let row = sqlx::query_as::<_, PortfolioHolding>(
            "INSERT INTO portfolio_holding (portfolio_id, isin, quantity, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (portfolio_id, isin) DO UPDATE
                SET quantity = portfolio_holding.quantity + EXCLUDED.quantity,
                    updated_at = now()
             RETURNING id, portfolio_id, isin, quantity, updated_at",
        )
        .bind(portfolio_id)
        .bind(isin)
        .bind(delta)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Получить все позиции портфеля.
    pub async fn get_holdings(&self, portfolio_id: i64) -> Result<Vec<PortfolioHolding>> {
        let rows = sqlx::query_as::<_, PortfolioHolding>(
            "SELECT id, portfolio_id, isin, quantity, updated_at
             FROM portfolio_holding
             WHERE portfolio_id = $1
             ORDER BY isin",
        )
        .bind(portfolio_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Получить позиции как HashMap<ISIN, quantity>.
    pub async fn get_holdings_map(&self, portfolio_id: i64) -> Result<HashMap<String, i64>> {
        let holdings = self.get_holdings(portfolio_id).await?;
        Ok(holdings.into_iter().map(|h| (h.isin, h.quantity)).collect())
    }

    // ── Денежные средства ──────────────────────────────────────

    /// Установить сумму свободных денег в портфеле (upsert).
    pub async fn set_cash(&self, portfolio_id: i64, amount: Decimal, currency: &str) -> Result<PortfolioCash> {
        let row = sqlx::query_as::<_, PortfolioCash>(
            "INSERT INTO portfolio_cash (portfolio_id, amount, currency, updated_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (portfolio_id) DO UPDATE
                SET amount = EXCLUDED.amount, currency = EXCLUDED.currency, updated_at = now()
             RETURNING id, portfolio_id, amount, currency, updated_at",
        )
        .bind(portfolio_id)
        .bind(amount)
        .bind(currency)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Изменить сумму свободных денег на delta.
    pub async fn adjust_cash(&self, portfolio_id: i64, delta: Decimal) -> Result<PortfolioCash> {
        let row = sqlx::query_as::<_, PortfolioCash>(
            "INSERT INTO portfolio_cash (portfolio_id, amount, currency, updated_at)
             VALUES ($1, $2, 'RUB', now())
             ON CONFLICT (portfolio_id) DO UPDATE
                SET amount = portfolio_cash.amount + EXCLUDED.amount,
                    updated_at = now()
             RETURNING id, portfolio_id, amount, currency, updated_at",
        )
        .bind(portfolio_id)
        .bind(delta)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Получить текущие денежные средства портфеля.
    pub async fn get_cash(&self, portfolio_id: i64) -> Result<Decimal> {
        let row = sqlx::query_as::<_, PortfolioCash>(
            "SELECT id, portfolio_id, amount, currency, updated_at
             FROM portfolio_cash
             WHERE portfolio_id = $1",
        )
        .bind(portfolio_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.amount).unwrap_or(Decimal::ZERO))
    }

    // ── Рыночная стоимость ─────────────────────────────────────

    /// Вычислить рыночную стоимость портфеля с заданными ценами.
    ///
    /// `prices` — HashMap<ISIN, цена одной облигации>.
    /// Возвращает (bonds_value, cash, total).
    pub async fn compute_market_value(
        &self,
        portfolio_id: i64,
        prices: &HashMap<String, Decimal>,
    ) -> Result<(Decimal, Decimal, Decimal)> {
        let holdings = self.get_holdings(portfolio_id).await?;
        let cash = self.get_cash(portfolio_id).await?;

        let mut bonds_value = Decimal::ZERO;
        for h in &holdings {
            if let Some(&price) = prices.get(&h.isin) {
                bonds_value += price * Decimal::from(h.quantity);
            }
        }
        let total = bonds_value + cash;
        Ok((bonds_value, cash, total))
    }

    // ── Снимки стоимости портфеля ──────────────────────────────

    /// Записать снимок рыночной стоимости портфеля на дату (upsert).
    pub async fn save_snapshot(
        &self,
        portfolio_id: i64,
        date: NaiveDate,
        market_value: Decimal,
        cash: Decimal,
        bonds_value: Decimal,
    ) -> Result<PortfolioSnapshot> {
        let row = sqlx::query_as::<_, PortfolioSnapshot>(
            "INSERT INTO portfolio_snapshot (portfolio_id, date, market_value, cash, bonds_value)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (portfolio_id, date) DO UPDATE
                SET market_value = EXCLUDED.market_value,
                    cash = EXCLUDED.cash,
                    bonds_value = EXCLUDED.bonds_value
             RETURNING id, portfolio_id, date, market_value, cash, bonds_value",
        )
        .bind(portfolio_id)
        .bind(date)
        .bind(market_value)
        .bind(cash)
        .bind(bonds_value)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Вычислить и сразу сохранить снимок стоимости портфеля.
    pub async fn record_snapshot(
        &self,
        portfolio_id: i64,
        date: NaiveDate,
        prices: &HashMap<String, Decimal>,
    ) -> Result<PortfolioSnapshot> {
        let (bonds_value, cash, total) = self.compute_market_value(portfolio_id, prices).await?;
        self.save_snapshot(portfolio_id, date, total, cash, bonds_value).await
    }

    /// Получить все снимки портфеля (для графика стоимости), отсортированные по дате.
    pub async fn get_snapshots(&self, portfolio_id: i64) -> Result<Vec<PortfolioSnapshot>> {
        let rows = sqlx::query_as::<_, PortfolioSnapshot>(
            "SELECT id, portfolio_id, date, market_value, cash, bonds_value
             FROM portfolio_snapshot
             WHERE portfolio_id = $1
             ORDER BY date ASC",
        )
        .bind(portfolio_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Получить снимки за период.
    pub async fn get_snapshots_range(
        &self,
        portfolio_id: i64,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<PortfolioSnapshot>> {
        let rows = sqlx::query_as::<_, PortfolioSnapshot>(
            "SELECT id, portfolio_id, date, market_value, cash, bonds_value
             FROM portfolio_snapshot
             WHERE portfolio_id = $1 AND date BETWEEN $2 AND $3
             ORDER BY date ASC",
        )
        .bind(portfolio_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Рассчитать итоговую доходность портфеля (простую) по снимкам.
    /// Возвращает `None`, если снимков меньше 2.
    pub async fn compute_total_return(&self, portfolio_id: i64) -> Result<Option<Decimal>> {
        let snapshots = self.get_snapshots(portfolio_id).await?;
        if snapshots.len() < 2 {
            return Ok(None);
        }
        let first = &snapshots[0].market_value;
        let last = &snapshots[snapshots.len() - 1].market_value;
        if first.is_zero() {
            return Ok(None);
        }
        Ok(Some((last - first) / first))
    }
}
