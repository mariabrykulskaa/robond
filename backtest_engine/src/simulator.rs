//! Ядро симулятора: обработка сделок, платежей и переоценки портфеля

use chrono::NaiveDate;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::HashMap;
use trading_strategies::{MarketOrder, MarketOrderType, Portfolio};

use crate::models::{PaymentEvent, TradeEvent};

/// (open, close, low, high, volume, facevalue, accint)
type PriceEntry = (f64, f64, f64, f64, f64, f64, f64);

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
    /// Кешированные цены: (дата, ISIN) -> (open, close, low, high, volume, facevalue, accint)
    pub price_cache: HashMap<(NaiveDate, String), PriceEntry>,
    /// Количество облигаций в портфеле: ISIN -> кол-во
    pub holdings: HashMap<String, i64>,
    /// Номиналы облигаций: ISIN -> номинал
    pub facevalues: HashMap<String, f64>,
    /// Объём выпуска: ISIN -> issue_volume (штук)
    pub issue_volumes: HashMap<String, i64>,
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
            issue_volumes: HashMap::new(),
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
        accint: f64,
    ) {
        let entry = (open, close, low, high, volume, facevalue, accint);
        self.price_cache.insert((self.current_date, isin.clone()), entry);
        self.last_known_price.insert(isin.clone(), entry);
        self.facevalues.entry(isin).or_insert(facevalue);
    }

    /// Обрабатывает рыночный ордер (покупка или продажа)
    /// Цена берётся как средняя между low и high в свечке
    pub fn execute_order(&mut self, order: MarketOrder, use_mid_price: bool) -> Result<TradeEvent, String> {
        let key = (self.current_date, order.isin.clone());

        let (_open, close, low, high, volume, facevalue, accint) = self
            .price_cache
            .get(&key)
            .copied()
            .ok_or_else(|| format!("Нет данных о цене для {} на {}", order.isin, self.current_date))?;

        // Если в этот день не было торгов — операции запрещены.
        if volume == 0.0 {
            return Err(format!("Нет торгов для {} на {}", order.isin, self.current_date));
        }

        // Ограничиваем покупку дневным объёмом торгов и объёмом выпуска.
        if order.order_type == MarketOrderType::Buy {
            let day_volume = volume as i64;
            if day_volume > 0 && order.count > day_volume {
                return Err(format!(
                    "Превышен дневной объём торгов для {}: запрошено {}, доступно {}",
                    order.isin, order.count, day_volume
                ));
            }
            if let Some(&issue_vol) = self.issue_volumes.get(&order.isin) {
                if issue_vol > 0 && order.count > issue_vol {
                    return Err(format!(
                        "Превышен объём выпуска для {}: запрошено {}, выпуск {}",
                        order.isin, order.count, issue_vol
                    ));
                }
            }
        }

        // Используем среднюю цену (середину между low и high)
        let execution_price = if use_mid_price { (low + high) / 2.0 } else { close };

        // Рассчитываем размер позиции в абсолютных рублях: цена облигации + НКД на единицу
        let price_per_unit = (execution_price / 100.0) * facevalue;
        let total_per_unit = price_per_unit + accint;
        let amount_in_rubles = total_per_unit * order.count as f64;
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
            accint,
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

    /// Принудительное погашение облигации по оферте (выкуп по номиналу).
    /// Зачисляет facevalue * quantity на счёт и обнуляет позицию.
    pub fn force_redeem_bond(&mut self, isin: &str) -> Option<PaymentEvent> {
        let quantity = *self.holdings.get(isin)?;
        if quantity == 0 {
            return None;
        }
        let facevalue = *self.facevalues.get(isin)?;
        let total_amount = facevalue * quantity as f64;

        self.portfolio.free_money += decimal_from_f64_option(total_amount)?;
        self.holdings.insert(isin.to_string(), 0);
        self.portfolio.bonds_count.insert(isin.to_string(), 0);

        let event = PaymentEvent {
            date: self.current_date,
            isin: isin.to_string(),
            quantity,
            amount_per_unit: facevalue,
            total_amount,
            payment_type: "offer_redemption".to_string(),
        };
        self.payments.push(event.clone());
        Some(event)
    }

    /// Списание облигации в 0 при дефолте. Позиция обнуляется без зачисления денег.
    pub fn write_off_bond(&mut self, isin: &str) -> Option<PaymentEvent> {
        let quantity = *self.holdings.get(isin)?;
        if quantity == 0 {
            return None;
        }
        self.holdings.insert(isin.to_string(), 0);
        self.portfolio.bonds_count.insert(isin.to_string(), 0);

        let event = PaymentEvent {
            date: self.current_date,
            isin: isin.to_string(),
            quantity,
            amount_per_unit: 0.0,
            total_amount: 0.0,
            payment_type: "default_write_off".to_string(),
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
                if let Some(&(_, close, _, _, _, facevalue, accint)) = price_entry {
                    let price_per_unit = (close / 100.0) * facevalue + accint;
                    total += price_per_unit * *quantity as f64;
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
