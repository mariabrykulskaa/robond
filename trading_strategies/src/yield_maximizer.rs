use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use financial::naive_date::xirr;
use rust_decimal::prelude::*;

use crate::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

/// Стратегия максимизации доходности.
///
/// Ключевые отличия от базовой стратегии:
/// - **Нет стоп-лосса** — держим до погашения, не кристаллизуем убытки.
///   Дефолтные облигации списываются движком автоматически (цена < 20%).
/// - **Hard deadline** — покупаем только бонды, которые погасятся до конца бэктеста,
///   чтобы XIRR реализовался полностью (без mark-to-market потерь на выходе).
/// - **Динамический порог** — если cash > 25%, снижаем min_yield чтобы деньги не простаивали.
/// - **Score-based ранжирование** — предпочитаем бонды с высоким XIRR и короткой дюрацией
///   (быстрый оборот капитала → больше compound эффект).
pub struct YieldMaximizerStrategy {
    /// Максимальная доля одной бумаги в портфеле
    pub max_weight: f64,
    /// Минимальный срок до погашения (дни)
    pub min_days_to_maturity: i64,
    /// Минимальная цена облигации (% от номинала) — фильтр мусора
    pub min_price_pct: f64,
    /// Максимальная цена облигации (% от номинала)
    pub max_price_pct: f64,
    /// Минимальная XIRR для покупки при нормальных условиях
    pub min_yield: f64,
    /// Максимальная XIRR — отсечка distressed бумаг (слишком высокая XIRR = риск дефолта)
    pub max_yield: f64,
    /// Минимальная XIRR при избытке кэша (> cash_urgency_threshold)
    pub min_yield_urgent: f64,
    /// Порог кэша для включения «urgent» режима (доля от портфеля)
    pub cash_urgency_threshold: f64,
    /// Минимальный дневной объём торгов для покупки (фильтр неликвида)
    pub min_volume_for_buy: i64,
    /// Крайняя дата: не покупаем бонды с погашением после этой даты.
    /// Должна быть <= end_date бэктеста (или чуть раньше для буфера).
    pub hard_deadline: NaiveDate,
}

impl Default for YieldMaximizerStrategy {
    fn default() -> Self {
        Self {
            max_weight: 0.05,
            min_days_to_maturity: 14,
            min_price_pct: 75.0,
            max_price_pct: 120.0,
            min_yield: 0.01,
            max_yield: 0.45,
            min_yield_urgent: 0.005,
            cash_urgency_threshold: 0.30,
            min_volume_for_buy: 50,
            hard_deadline: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
        }
    }
}

fn bond_xirr(buy_price: Decimal, current_date: NaiveDate, info: &BondPersistentInfo) -> f64 {
    let buy = buy_price.to_f64().unwrap_or(0.0);
    if buy <= 0.0 {
        return -1.0;
    }
    let mut cash_flow = vec![-buy];
    let mut dates = vec![current_date];
    for p in &info.payments {
        if p.date > current_date + Duration::days(3) {
            let amt = p.amount.to_f64().unwrap_or(0.0);
            if amt > 0.0 {
                cash_flow.push(amt);
                dates.push(p.date);
            }
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

impl Strategy for YieldMaximizerStrategy {
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
        let pv_f64 = portfolio_value.to_f64().unwrap_or(1.0);

        let mut orders = Vec::new();

        // НЕТ стоп-лосса. Дефолты обрабатываются движком (write_off при цене < 20%).
        // Держим до погашения — не кристаллизуем бумажные убытки.

        // ── Текущие веса позиций ────────────────────────────────────────
        let mut current_weights: HashMap<&Isin, f64> = HashMap::new();
        for (isin, &count) in &portfolio.bonds_count {
            if count <= 0 {
                continue;
            }
            if let Some(price) = bonds_prices.get(isin) {
                let pos_value = price.to_f64().unwrap_or(0.0) * count as f64;
                current_weights.insert(isin, pos_value / pv_f64);
            }
        }

        // ── Динамический порог XIRR ─────────────────────────────────────
        let cash_ratio = portfolio.free_money.to_f64().unwrap_or(0.0) / pv_f64;
        let effective_min_yield = if cash_ratio > self.cash_urgency_threshold {
            self.min_yield_urgent
        } else {
            self.min_yield
        };

        // ── Кандидаты на покупку ─────────────────────────────────────────
        struct Candidate {
            isin: Isin,
            yield_xirr: f64,
            price: Decimal,
            max_buy: i64,
            days_to_maturity: i64,
        }

        let mut candidates: Vec<Candidate> = Vec::new();

        for (isin, info) in bonds_info {
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

            // Hard deadline — не покупаем бонды, которые не погасятся до конца бэктеста
            if maturity > self.hard_deadline {
                continue;
            }

            let days_to_maturity = (maturity - current_date).num_days();
            if days_to_maturity < self.min_days_to_maturity {
                continue;
            }

            let yield_xirr = bond_xirr(price, current_date, info);
            if yield_xirr < effective_min_yield {
                continue;
            }
            if yield_xirr > self.max_yield {
                continue; // distressed — подозрительно высокая доходность
            }

            let day_vol = bonds_volumes.get(isin).copied().unwrap_or(0);
            if day_vol < self.min_volume_for_buy {
                continue; // неликвид — большие спреды, ненадёжные цены
            }

            candidates.push(Candidate {
                isin: isin.clone(),
                yield_xirr,
                price,
                max_buy: day_vol,
                days_to_maturity,
            });
        }

        // Score: XIRR с бонусом за короткий срок (быстрый оборот → компаундинг)
        // score = XIRR * (1 + 0.5 * (1 - days/365))  для days < 365
        // Т.е. бонд на 6 месяцев с XIRR 20% получает score ~24%, а на 12 мес — 20%.
        candidates.sort_by(|a, b| {
            let score_a = a.yield_xirr * (1.0 + 0.5 * (1.0 - (a.days_to_maturity as f64 / 365.0)).max(0.0));
            let score_b = b.yield_xirr * (1.0 + 0.5 * (1.0 - (b.days_to_maturity as f64 / 365.0)).max(0.0));
            score_b.partial_cmp(&score_a).unwrap()
        });

        // ── Покупаем ────────────────────────────────────────────────────
        let mut free_money = portfolio.free_money;
        let max_position_value = pv_f64 * self.max_weight;

        for cand in &candidates {
            if free_money <= Decimal::ZERO {
                break;
            }

            let existing_weight = current_weights.get(&cand.isin).copied().unwrap_or(0.0);
            let existing_value = existing_weight * pv_f64;
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
                // Обновляем вес для следующих итераций
                let w = current_weights.entry(&cand.isin).or_insert(0.0);
                *w += (cost.to_f64().unwrap_or(0.0)) / pv_f64;
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
