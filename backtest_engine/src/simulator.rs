//! Ядро симулятора: обработка сделок, платежей и переоценки портфеля

use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use trading_strategies::{MarketOrder, MarketOrderType, Portfolio};

use crate::models::{PaymentEvent, TradeEvent};

/// (open, close, low, high, volume, facevalue)
type PriceEntry = (f64, f64, f64, f64, f64, f64);

/// Основной симулятор рынка и портфеля
pub struct MarketSimulator {
    /// Текущий день симуляции
    pub current_date: NaiveDate,
    /// Портфель инвестора
    pub portfolio: Portfolio,
    /// История сделок
    pub trades: Vec<TradeEvent>,
    /// История платежей
    pub payments: Vec<PaymentEvent>,
    /// Кешированные цены: (дата, ISIN) -> (open, close, low, high, volume, facevalue)
    pub price_cache: HashMap<(NaiveDate, String), PriceEntry>,
    /// Количество облигаций в портфеле: ISIN -> кол-во
    pub holdings: HashMap<String, i64>,
    /// Номиналы облигаций: ISIN -> номинал
    pub facevalues: HashMap<String, f64>,
    /// Последняя известная цена для каждой облигации (для оценки портфеля в нерабочие дни)
    last_known_price: HashMap<String, PriceEntry>,
}

impl MarketSimulator {
    /// Создаёт новый симулятор с начальным капиталом
    pub fn new(initial_capital: Decimal, start_date: NaiveDate) -> Self {
        Self {
            current_date: start_date,
            portfolio: Portfolio {
                free_money: initial_capital,
                bonds_count: HashMap::new(),
            },
            trades: Vec::new(),
            payments: Vec::new(),
            price_cache: HashMap::new(),
            holdings: HashMap::new(),
            facevalues: HashMap::new(),
            last_known_price: HashMap::new(),
        }
    }

    /// Обновляет дату и загружает данные на новую дату
    pub fn set_date(&mut self, date: NaiveDate) {
        self.current_date = date;
    }

    /// Кеширует цены для дня
    #[allow(clippy::too_many_arguments)]
    pub fn cache_prices(
        &mut self,
        isin: String,
        open: f64,
        close: f64,
        low: f64,
        high: f64,
        volume: f64,
        facevalue: f64,
    ) {
        let entry = (open, close, low, high, volume, facevalue);
        self.price_cache.insert((self.current_date, isin.clone()), entry);
        self.last_known_price.insert(isin.clone(), entry);
        self.facevalues.entry(isin).or_insert(facevalue);
    }

    /// Обрабатывает рыночный ордер (покупка или продажа)
    /// Цена берётся как средняя между low и high в свечке
    pub fn execute_order(&mut self, order: MarketOrder, use_mid_price: bool) -> Result<TradeEvent, String> {
        let key = (self.current_date, order.isin.clone());

        let (_open, close, low, high, _volume, facevalue) = self
            .price_cache
            .get(&key)
            .copied()
            .ok_or_else(|| format!("Нет данных о цене для {} на {}", order.isin, self.current_date))?;

        // Используем среднюю цену (середину между low и high)
        let execution_price = if use_mid_price { (low + high) / 2.0 } else { close };

        // Рассчитываем размер позиции в абсолютных рублях (используя расчётную стоимость)
        let amount_in_rubles = (execution_price / 100.0) * facevalue * order.count as f64;
        let amount_decimal = decimal_from_f64(amount_in_rubles)?;

        match order.order_type {
            MarketOrderType::Buy => {
                if self.portfolio.free_money < amount_decimal {
                    return Err("Недостаточно средств для покупки".to_string());
                }

                self.portfolio.free_money -= amount_decimal;
                *self.holdings.entry(order.isin.clone()).or_insert(0) += order.count;
                self.portfolio
                    .bonds_count
                    .insert(order.isin.clone(), *self.holdings.get(&order.isin).unwrap());
            }
            MarketOrderType::Sell => {
                let current_holding = *self.holdings.get(&order.isin).unwrap_or(&0);
                if current_holding < order.count {
                    return Err(format!(
                        "Недостаточно облигаций {} для продажи ({} запрошено, {} в портфеле)",
                        order.isin, order.count, current_holding
                    ));
                }

                self.portfolio.free_money += amount_decimal;
                *self.holdings.entry(order.isin.clone()).or_insert(0) -= order.count;
                self.portfolio
                    .bonds_count
                    .insert(order.isin.clone(), *self.holdings.get(&order.isin).unwrap());
            }
        }

        let event = TradeEvent {
            date: self.current_date,
            isin: order.isin,
            quantity: order.count,
            price: execution_price,
            total_amount: amount_in_rubles,
            side: match order.order_type {
                MarketOrderType::Buy => "buy".to_string(),
                MarketOrderType::Sell => "sell".to_string(),
            },
        };
        self.trades.push(event.clone());
        Ok(event)
    }

    /// Обрабатывает выплату по облигации (купон или погашение)
    pub fn process_payment(
        &mut self,
        isin: String,
        amount_percent: f64, // в % от номинала
        payment_type: String,
    ) -> Option<PaymentEvent> {
        let quantity = *self.holdings.get(&isin)?;
        if quantity == 0 {
            return None;
        }

        let facevalue = *self.facevalues.get(&isin)?;
        let amount_per_unit = (amount_percent / 100.0) * facevalue;
        let total_amount = amount_per_unit * quantity as f64;

        self.portfolio.free_money += decimal_from_f64_option(total_amount)?;

        let event = PaymentEvent {
            date: self.current_date,
            isin,
            quantity,
            amount_per_unit,
            total_amount,
            payment_type,
        };

        self.payments.push(event.clone());
        Some(event)
    }

    /// Оценивает текущий портфель по рыночным ценам.
    /// Если в текущий день нет свечи — использует последнюю известную цену закрытия.
    pub fn get_portfolio_value(&self) -> f64 {
        let mut total = self.portfolio.free_money.to_f64().unwrap_or(0.0);

        for (isin, quantity) in &self.holdings {
            if *quantity > 0 {
                let key = (self.current_date, isin.clone());
                let price_entry = self.price_cache.get(&key).or_else(|| self.last_known_price.get(isin));
                if let Some((_, close, _, _, _, facevalue)) = price_entry {
                    let position_value = (close / 100.0) * facevalue * *quantity as f64;
                    total += position_value;
                }
            }
        }

        total
    }

    /// Получает текущее состояние портфеля
    pub fn get_portfolio_snapshot(&self) -> crate::models::PortfolioSnapshot {
        let total = self.get_portfolio_value();
        let bonds_value = total - self.portfolio.free_money.to_f64().unwrap_or(0.0);
        crate::models::PortfolioSnapshot {
            date: self.current_date,
            cash: self.portfolio.free_money,
            positions: self.holdings.clone(),
            portfolio_value: bonds_value,
            total_value: total,
        }
    }
}

fn decimal_from_f64(value: f64) -> Result<Decimal, String> {
    value
        .to_string()
        .parse::<Decimal>()
        .map_err(|_| format!("Не удалось преобразовать сумму в Decimal: {value}"))
}

fn decimal_from_f64_option(value: f64) -> Option<Decimal> {
    value.to_string().parse::<Decimal>().ok()
}
