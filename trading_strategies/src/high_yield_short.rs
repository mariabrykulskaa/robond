use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use financial::naive_date::xirr;
use rust_decimal::prelude::*;

use crate::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

/// Агрессивная стратегия коротких высокодоходных облигаций.
///
/// Идея: покупаем облигации с XIRR >= 15%, срок до погашения до 2 лет.
/// Держим до погашения — не продаём по XIRR (доходность "залочена" при покупке).
/// Продаём только по стоп-лоссу (вероятный дефолт).
/// Активно реинвестируем погашения и купоны в новые бумаги.
pub struct HighYieldShortStrategy {
    /// Максимальная доля одной бумаги в портфеле
    pub max_weight: f64,
    /// Минимальный срок до погашения (дни)
    pub min_days_to_maturity: i64,
    /// Максимальный срок до погашения (дни)
    pub max_days_to_maturity: i64,
    /// Минимальная цена облигации (% от номинала)
    pub min_price_pct: f64,
    /// Максимальная цена облигации (% от номинала)
    pub max_price_pct: f64,
    /// Порог стоп-лосса (% от номинала)
    pub stop_loss_pct: f64,
    /// Минимальная XIRR для покупки
    pub min_yield: f64,
}

impl Default for HighYieldShortStrategy {
    fn default() -> Self {
        Self {
            max_weight: 0.08,
            min_days_to_maturity: 14,
            max_days_to_maturity: 400,
            min_price_pct: 60.0,
            max_price_pct: 120.0,
            stop_loss_pct: 70.0,
            min_yield: 0.22,
        }
    }
}

fn bond_xirr(buy_price: Decimal, current_date: NaiveDate, info: &BondPersistentInfo) -> f64 {
    let mut cash_flow = vec![-buy_price.as_f64()];
    let mut dates = vec![current_date];
    for p in &info.payments {
        if p.date > current_date + Duration::days(5) {
            cash_flow.push(p.amount.as_f64());
            dates.push(p.date);
        }
    }
    if cash_flow.len() < 2 {
        return -1.0;
    }
    xirr(&cash_flow, &dates, None).unwrap_or(-1.0)
}

fn effective_maturity(info: &BondPersistentInfo) -> Option<NaiveDate> {
    match (info.bond_info.maturity_date, info.bond_info.offer_date) {
        (Some(m), Some(o)) if o < m => Some(o),
        (Some(m), _) => Some(m),
        (None, Some(o)) => Some(o),
        _ => None,
    }
}

impl Strategy for HighYieldShortStrategy {
    fn decide_trades(
        &self,
        current_date: NaiveDate,
        portfolio: &Portfolio,
        bonds_info: &HashMap<Isin, BondPersistentInfo>,
        bonds_prices: &HashMap<Isin, Decimal>,
        bonds_volumes: &HashMap<Isin, i64>,
    ) -> Vec<MarketOrder> {
        let portfolio_value = portfolio.market_price(bonds_prices);
        if portfolio_value <= Decimal::ZERO {
            return vec![];
        }

        let mut orders = Vec::new();

        // ── Шаг 1: продаём только по стоп-лоссу (вероятный дефолт) ─────────
        for (isin, &count) in &portfolio.bonds_count {
            if count <= 0 {
                continue;
            }
            let Some(price) = bonds_prices.get(isin) else {
                continue;
            };
            let price_f64 = price.to_f64().unwrap_or(0.0);

            let facevalue = bonds_info
                .get(isin)
                .and_then(|i| i.bond_info.facevalue)
                .unwrap_or(1000.0);
            let price_pct = if facevalue > 0.0 {
                (price_f64 / facevalue) * 100.0
            } else {
                100.0
            };

            if price_pct < self.stop_loss_pct && price_pct > 0.0 {
                orders.push(MarketOrder {
                    isin: isin.clone(),
                    order_type: MarketOrderType::Sell,
                    count,
                });
            }
        }

        // ── Шаг 2: текущие веса ───────────────────────────────────────────
        let mut current_weights: HashMap<&Isin, f64> = HashMap::new();
        for (isin, &count) in &portfolio.bonds_count {
            if count <= 0 {
                continue;
            }
            if let Some(price) = bonds_prices.get(isin) {
                let pos_value = price.to_f64().unwrap_or(0.0) * count as f64;
                let pv = portfolio_value.to_f64().unwrap_or(1.0);
                current_weights.insert(isin, pos_value / pv);
            }
        }

        // ── Шаг 3: кандидаты на покупку ────────────────────────────────────
        struct Candidate {
            isin: Isin,
            yield_xirr: f64,
            price: Decimal,
            max_buy: i64,
        }

        let mut candidates: Vec<Candidate> = Vec::new();

        for (isin, info) in bonds_info {
            if orders.iter().any(|o| o.isin == *isin) {
                continue;
            }
            if info.bond_info.is_subordinated == Some(true) {
                continue;
            }
            let Some(&price) = bonds_prices.get(isin) else {
                continue;
            };
            if price <= Decimal::ZERO {
                continue;
            }
            if info.bond_info.default_date.is_some() {
                continue;
            }

            let facevalue = info.bond_info.facevalue.unwrap_or(1000.0);
            let price_f64 = price.to_f64().unwrap_or(0.0);
            let price_pct = if facevalue > 0.0 {
                (price_f64 / facevalue) * 100.0
            } else {
                100.0
            };

            if price_pct < self.min_price_pct || price_pct > self.max_price_pct {
                continue;
            }

            let Some(maturity) = effective_maturity(info) else {
                continue;
            };
            let days_to_maturity = (maturity - current_date).num_days();
            if days_to_maturity < self.min_days_to_maturity || days_to_maturity > self.max_days_to_maturity {
                continue;
            }

            let yield_xirr = bond_xirr(price, current_date, info);
            if yield_xirr < self.min_yield {
                continue;
            }

            let day_vol = bonds_volumes.get(isin).copied().unwrap_or(0);
            if day_vol <= 0 {
                continue;
            }

            candidates.push(Candidate {
                isin: isin.clone(),
                yield_xirr,
                price,
                max_buy: day_vol,
            });
        }

        // Сортируем: лучшая доходность первой
        candidates.sort_by(|a, b| b.yield_xirr.partial_cmp(&a.yield_xirr).unwrap());

        // ── Шаг 4: покупаем ────────────────────────────────────────────────
        let mut free_money = portfolio.free_money;
        for o in &orders {
            if o.order_type == MarketOrderType::Sell
                && let Some(p) = bonds_prices.get(&o.isin)
            {
                free_money += *p * Decimal::from(o.count);
            }
        }

        let max_position_value = portfolio_value.to_f64().unwrap_or(0.0) * self.max_weight;

        for cand in &candidates {
            if free_money <= Decimal::ZERO {
                break;
            }

            let existing_weight = current_weights.get(&cand.isin).copied().unwrap_or(0.0);
            let existing_value = existing_weight * portfolio_value.to_f64().unwrap_or(0.0);
            let room = max_position_value - existing_value;
            if room <= 0.0 {
                continue;
            }

            let price_f64 = cand.price.to_f64().unwrap_or(1.0);
            if price_f64 <= 0.0 {
                continue;
            }

            let count_by_weight = (room / price_f64).floor() as i64;
            let count_by_money = (free_money / cand.price).to_i64().unwrap_or(0);
            let count = count_by_weight.min(count_by_money).min(cand.max_buy);

            if count > 0 {
                let cost = cand.price * Decimal::from(count);
                free_money -= cost;
                orders.push(MarketOrder {
                    isin: cand.isin.clone(),
                    order_type: MarketOrderType::Buy,
                    count,
                });
            }
        }

        orders
    }
}
