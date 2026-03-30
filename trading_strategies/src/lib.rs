//! Модуль с торговыми стратегиями
//!
//! TODO: добавить хотя бы одну торговую стратегию
//!

pub mod strategies;

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Идентификатор бумаги (ISIN).
pub type Isin = String;

/// Состояние портфеля
#[derive(Debug, Clone)]
pub struct Portfolio {
    /// Свободная денежная сумма
    pub free_money: Decimal,
    /// Количество облигаций в портфеле
    pub bonds_count: HashMap<Isin, i64>,
}

impl Portfolio {
    fn market_price(&self, bonds_prices: &HashMap<Isin, Decimal>) -> Decimal {
        let mut price = self.free_money;
        for (isin, &count) in self.bonds_count.iter() {
            match bonds_prices.get(isin) {
                None => {}
                Some(bond_price) => price += Decimal::from(count) * bond_price,
            }
        }
        price
    }
}

/// Тип выплаты по облигации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentType {
    /// Купонная выплата
    Coupon,
    /// Амортизация (частичное погашение номинала)
    Amortization,
    /// Полное погашение номинала
    Redemption,
}

/// Информация об одной выплате по облигации
#[derive(Debug, Clone)]
pub struct PaymentInfo {
    /// Дата выплаты
    pub date: NaiveDate,
    /// Сумма выплаты
    pub amount: Decimal,
    /// Тип выплаты
    pub payment_type: PaymentType,
}

/// Общая информация об облигации (из таблицы bond_bond)
#[derive(Debug, Clone)]
pub struct BondCommonInfo {
    /// ISIN код облигации
    pub isin: String,
    /// ID валюты (для фильтрации не-рублёвых облигаций)
    pub currency_id: Option<i64>,
    /// Название облигации
    pub title: Option<String>,
    /// Является ли облигация субординированной
    pub is_subordinated: Option<bool>,
    /// Объём выпуска
    pub issue_volume: Option<i64>,
    /// Дата размещения
    pub placement_date: Option<NaiveDate>,
    /// Дата погашения
    pub maturity_date: Option<NaiveDate>,
    /// Номинальная стоимость
    pub facevalue: Option<f64>,
    /// Начальная номинальная стоимость
    pub start_facevalue: Option<f64>,
    /// Режим торгов (board)
    pub board: Option<String>,
    /// Для квалифицированных инвесторов
    pub is_for_qualified_investors: Option<bool>,
    /// Торгуется ли в данный момент
    pub is_traded: bool,
    /// Дата оферты (если есть) — считаем виртуальной датой погашения
    pub offer_date: Option<NaiveDate>,
    /// Размер текущего купона (руб.)
    pub coupon_size: Option<f64>,
    /// Периодичность купона (дней)
    pub coupon_period: Option<i16>,
    /// Текущий НКД по купону (руб.)
    pub coupon_aci: Option<f64>,
}

/// Не меняющаяся информация об облигации
#[derive(Debug, Clone)]
pub struct BondPersistentInfo {
    /// Общая информация об облигации
    pub bond_info: BondCommonInfo,
    /// Все выплаты: купоны, амортизации и погашения.
    pub payments: Vec<PaymentInfo>,
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
        bonds_prices: &HashMap<Isin, Decimal>,
    ) -> Vec<MarketOrder>;
}
