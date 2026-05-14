use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Портфель пользователя.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Portfolio {
    pub id: i64,
    pub name: String,
    pub user_id: Option<i64>,
    pub strategy_name: Option<String>,
    pub strategy_running: Option<bool>,
    pub pending_strategy_run: bool,
    pub created_at: DateTime<Utc>,
}

/// Позиция: количество облигаций определённого типа в портфеле.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortfolioHolding {
    pub id: i64,
    pub portfolio_id: i64,
    pub isin: String,
    pub quantity: i64,
    pub updated_at: DateTime<Utc>,
}

/// Свободные денежные средства в портфеле.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortfolioCash {
    pub id: i64,
    pub portfolio_id: i64,
    pub amount: Decimal,
    pub currency: String,
    pub updated_at: DateTime<Utc>,
}

/// Снимок рыночной стоимости портфеля на определённую дату.
/// Используется для построения графика стоимости портфеля и расчёта доходности.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PortfolioSnapshot {
    pub id: i64,
    pub portfolio_id: i64,
    pub date: NaiveDate,
    pub market_value: Decimal,
    pub cash: Decimal,
    pub bonds_value: Decimal,
}
