use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Информация о купоне облигации
/// Соответствует таблице bond_coupon
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BondCoupon {
    /// ID записи купона
    pub id: i64,

    /// Текстовое описание купона
    pub description: Option<String>,

    /// Размер купона
    pub size: Option<f32>,

    /// Накопленный купонный доход
    pub aci: Option<f32>,

    /// Период купона в днях
    pub period: Option<i16>,

    /// Тип купона
    pub type_id: Option<i64>,

    /// Сумма выплаты
    pub sum: Option<f32>,
}

/// Выплата по облигации
/// Соответствует таблице bond_payment
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BondPayment {
    /// ID записи выплаты
    pub id: i64,

    /// Дата выплаты
    pub date: Option<NaiveDate>,

    /// Сумма выплаты в валюте выпуска
    pub size: Option<f32>,

    /// Относительный размер выплаты в процентах
    pub relative_size: Option<f32>,

    /// ID облигации
    pub bond_id: Option<i64>,

    /// ID валюты
    pub currency_id: Option<i64>,

    /// ID типа выплаты
    pub type_id: Option<i64>,
}

/// Исторические данные по облигации (свечи)
/// Соответствует таблице bond_bondhistorydata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BondHistoryData {
    /// ID записи
    pub id: i64,

    /// Дата торгов
    pub date: NaiveDate,

    /// Количество сделок
    pub num_trades: Option<f64>,

    /// Объем торгов в валюте
    pub value: Option<f64>,

    /// Минимальная цена за день (% от номинала)
    pub low: Option<f64>,

    /// Максимальная цена за день (% от номинала)
    pub high: Option<f64>,

    /// Цена закрытия (% от номинала)
    pub close: Option<f64>,

    /// Цена открытия (% от номинала)
    pub open: Option<f64>,

    /// Объем торгов в штуках
    pub volume: Option<f64>,

    /// Номинальная стоимость
    pub facevalue: Option<f64>,

    /// Накопленный купонный доход
    pub accint: Option<f64>,

    /// Полная информация из MOEX в формате JSON
    pub full_information: Option<JsonValue>,

    /// ID облигации (внешний ключ на bond_bond)
    pub bond_id: i64,
}

/// Информация об облигации
/// Соответствует таблице bond_bond
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BondInfo {
    /// ID облигации
    pub id: i64,

    /// ISIN код облигации
    pub isin: Option<String>,

    /// Название облигации
    pub title: Option<String>,

    /// Является ли облигация субординированной
    pub is_subordinated: Option<bool>,

    /// Объем выпуска
    pub issue_volume: Option<i64>,

    /// Дата размещения
    pub placement_date: Option<NaiveDate>,

    /// Дата погашения
    pub maturity_date: Option<NaiveDate>,

    /// Обеспечение
    pub collateral: Option<String>,

    /// ID на MOEX
    pub moex_id: Option<i64>,

    /// Торгуется ли в Тинькофф Инвестициях
    pub is_traded_in_ti: Option<bool>,

    /// Сайт эмитента
    pub website: Option<String>,

    /// Текущая доходность
    pub current_yield: Option<f32>,

    /// Доходность к погашению
    pub yield_to_maturity: Option<f32>,

    /// ID купона
    pub coupon_id: Option<i64>,

    /// ID валюты
    pub currency_id: Option<i64>,

    /// ID эмитента
    pub emitter_id: Option<i64>,

    /// Режим торгов (board)
    pub board: Option<String>,

    /// Номинальная стоимость
    pub facevalue: Option<f32>,

    /// Текущая цена
    pub price: Option<f32>,

    /// Начальная номинальная стоимость
    pub start_facevalue: Option<f32>,

    /// Дата окончания
    pub end_date: Option<NaiveDate>,

    /// ID ордера
    pub order_id: Option<i64>,

    /// Для квалифицированных инвесторов
    pub is_for_qualified_investors: Option<bool>,

    /// Ликвидность
    pub liquidity: Option<i64>,

    /// Торгуется ли в данный момент
    pub is_traded: bool,

    /// Полная информация из MOEX в формате JSON
    pub full_moex_information: Option<JsonValue>,
}
