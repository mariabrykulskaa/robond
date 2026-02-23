//! Модели данных для симуляции

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Результат одной торговой операции
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEvent {
    /// Дата сделки
    pub date: NaiveDate,
    /// ISIN облигации
    pub isin: String,
    /// Количество единиц
    pub quantity: i64,
    /// Цена за единицу (% от номинала)
    pub price: f64,
    /// Общая сумма сделки (в рублях)
    pub total_amount: f64,
    /// Тип сделки: "buy" или "sell"
    pub side: String,
}

/// Событие выплаты (купон или погашение номинала)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    /// Дата выплаты
    pub date: NaiveDate,
    /// ISIN облигации
    pub isin: String,
    /// Количество облигаций, по которым была выплата
    pub quantity: i64,
    /// Размер выплаты на единицу (в % от номинала)
    pub amount_per_unit: f64,
    /// Общая сумма выплаты
    pub total_amount: f64,
    /// Тип выплаты: "coupon" или "redemption"
    pub payment_type: String,
}

/// Снимок портфеля на конкретную дату
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    /// Дата снимка
    pub date: NaiveDate,
    /// Свободная денежная сумма
    pub cash: Decimal,
    /// Состав портфеля (ISIN -> количество)
    pub positions: HashMap<String, i64>,
    /// Оценочная стоимость портфеля по market price
    pub portfolio_value: f64,
    /// Общая стоимость (cash + портфель)
    pub total_value: f64,
}

/// Полный результат бэктеста
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    /// Начальный капитал
    pub initial_capital: Decimal,
    /// Финальная стоимость портфеля
    pub final_value: f64,
    /// Прибыль/убыток
    pub profit_loss: f64,
    /// Процент возврата
    pub return_percent: f64,
    /// Все торговые события
    pub trades: Vec<TradeEvent>,
    /// Все события выплат
    pub payments: Vec<PaymentEvent>,
    /// Снимки портфеля на важные даты
    pub portfolio_snapshots: Vec<PortfolioSnapshot>,
    /// Дата начала симуляции
    pub start_date: NaiveDate,
    /// Дата окончания симуляции
    pub end_date: NaiveDate,
}

/// Симуляция сделки (результат стратегии)
#[derive(Debug, Clone)]
pub struct TradeSimulation {
    /// Рыночное поручение от стратегии
    pub order: trading_strategies::MarketOrder,
    /// Выполнена ли сделка
    pub executed: bool,
    /// Цена исполнения (% от номинала)
    pub execution_price: Option<f64>,
    /// Причина неисполнения (если applicable)
    pub failure_reason: Option<String>,
}
