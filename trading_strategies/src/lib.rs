//! Модуль с торговыми стратегиями
//!
//! TODO: добавить хотя бы одну торговую стратегию

use std::collections::HashMap;

use chrono::NaiveDate;

/// Денежная сумма в некоторых единицах.
///
/// TODO: определиться с тем, что это за единицы
pub type Money = i64;

/// Идентификатор бумаги (ISIN).
pub type Isin = String;

/// Состояние портфеля
#[derive(Debug, Clone)]
pub struct Portfolio {
    /// Свободная денежная сумма
    pub free_money: Money,
    /// Количество облигаций в портфеле
    pub bonds_count: HashMap<Isin, i64>,
}

/// Не меняющаяся информация об облигации
#[derive(Debug, Clone)]
pub struct BondPersistentInfo {
    /// Выплаты по купонам и номиналу: дата и сумма.
    pub payments: Vec<(NaiveDate, Money)>,
}

/// Тип рыночного торгового поручения
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketOrderType {
    /// Покупка
    Buy,
    /// Продажа
    Sell,
}

/// Рыночное торговое поручение: какую облигацию и в каком объёме купить или продать
#[derive(Debug, Clone)]
pub struct MarketOrder {
    pub isin: Isin,
    pub order_type: MarketOrderType,
    /// Количество штук (облигаций)
    pub count: i64,
}

/// Торговая стратегия
pub trait Strategy {
    /// По состоянию на конкретную дату решает, какие сделки совершить.
    fn decide_trades(
        &self,
        current_date: NaiveDate,
        portfolio: &Portfolio,
        bonds_info: &HashMap<Isin, BondPersistentInfo>,
        bonds_prices: &HashMap<Isin, Money>,
    ) -> Vec<MarketOrder>;
}
