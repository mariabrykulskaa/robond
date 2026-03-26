use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use financial::naive_date::xirr;
use rust_decimal::prelude::*;

use crate::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

/// Диверсифицированная стратегия коротких облигаций.
///
/// ## Идея
///
/// Покупаем облигации с коротким сроком до погашения (3–18 месяцев),
/// торгующиеся ниже номинала — т.е. с дисконтом. При погашении облигация
/// возвращается по номиналу, и этот дисконт превращается в гарантированную
/// прибыль. Плюс по дороге мы получаем купоны.
///
/// ## Защита от рисков
///
/// - **Диверсификация**: максимум 15% портфеля в одну бумагу, минимум 5 разных
///   облигаций — один дефолт не уничтожит портфель.
/// - **Короткий срок**: чем ближе погашение, тем меньше неопределённость.
/// - **Фильтр мусора**: исключаем облигации с ценой < 50% номинала (вероятный
///   дефолт), субординированные (списываются первыми при проблемах эмитента).
/// - **Ликвидность**: не покупаем больше дневного объёма торгов.
/// - **Низкий оборот**: покупаем и держим до погашения, продаём только если
///   рейтинг бумаги ухудшился (цена упала ниже 70%).
pub struct DiversifiedShortDurationStrategy {
    /// Максимальная доля одной бумаги в портфеле (0.15 = 15%)
    pub max_weight: f64,
    /// Минимальный срок до погашения (дни)
    pub min_days_to_maturity: i64,
    /// Максимальный срок до погашения (дни)
    pub max_days_to_maturity: i64,
    /// Минимальная цена облигации (% от номинала) — фильтр мусора
    pub min_price_pct: f64,
    /// Порог стоп-лосса: продаём, если цена упала ниже этого уровня (% от номинала)
    pub stop_loss_pct: f64,
    /// Минимальная доходность XIRR для покупки
    pub min_yield: f64,
}

impl Default for DiversifiedShortDurationStrategy {
    fn default() -> Self {
        Self {
            max_weight: 0.15,
            min_days_to_maturity: 90,
            max_days_to_maturity: 540,
            min_price_pct: 50.0,
            stop_loss_pct: 70.0,
            min_yield: 0.05,
        }
    }
}

/// Рассчитывает XIRR доходность облигации.
fn bond_xirr(
    buy_price: Decimal,
    current_date: NaiveDate,
    info: &BondPersistentInfo,
) -> f64 {
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

/// Эффективная дата погашения: оферта (если раньше) или матурити.
fn effective_maturity(info: &BondPersistentInfo) -> Option<NaiveDate> {
    match (info.bond_info.maturity_date, info.bond_info.offer_date) {
        (Some(m), Some(o)) if o < m => Some(o),
        (Some(m), _) => Some(m),
        (None, Some(o)) => Some(o),
        _ => None,
    }
}

impl Strategy for DiversifiedShortDurationStrategy {
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

        // ── Шаг 1: стоп-лосс — продаём бумаги, которые сильно упали ───────
        for (isin, &count) in &portfolio.bonds_count {
            if count <= 0 {
                continue;
            }
            if let Some(price) = bonds_prices.get(isin) {
                let price_f64 = price.to_f64().unwrap_or(0.0);
                // Цена — это абсолютная стоимость одной облигации (в рублях).
                // Нужно перевести в % от номинала для сравнения.
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
        }

        // ── Шаг 2: подсчитаем текущие веса позиций ────────────────────────
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

        // ── Шаг 3: отбираем кандидатов на покупку ──────────────────────────
        struct Candidate {
            isin: Isin,
            yield_xirr: f64,
            price: Decimal,
            max_buy: i64,
        }

        let mut candidates: Vec<Candidate> = Vec::new();

        for (isin, info) in bonds_info {
            // Пропускаем то, что уже продаём по стоп-лоссу.
            if orders.iter().any(|o| o.isin == *isin) {
                continue;
            }
            // Пропускаем субординированные.
            if info.bond_info.is_subordinated == Some(true) {
                continue;
            }
            // Пропускаем, если нет цены сегодня.
            let Some(&price) = bonds_prices.get(isin) else {
                continue;
            };
            if price <= Decimal::ZERO {
                continue;
            }
            // Пропускаем облигации с известным дефолтом.
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

            // Фильтр по цене: не берём мусор и не берём сильно выше номинала.
            if price_pct < self.min_price_pct || price_pct > 105.0 {
                continue;
            }

            // Фильтр по сроку до погашения.
            let Some(maturity) = effective_maturity(info) else {
                continue;
            };
            let days_to_maturity = (maturity - current_date).num_days();
            if days_to_maturity < self.min_days_to_maturity
                || days_to_maturity > self.max_days_to_maturity
            {
                continue;
            }

            // Фильтр по доходности XIRR.
            let yield_xirr = bond_xirr(price, current_date, info);
            if yield_xirr < self.min_yield {
                continue;
            }

            // Ограничение по объёму торгов.
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

        // Сортируем по доходности (лучшие — первыми).
        candidates.sort_by(|a, b| b.yield_xirr.partial_cmp(&a.yield_xirr).unwrap());

        // ── Шаг 4: покупаем, распределяя деньги по кандидатам ──────────────
        let mut free_money = portfolio.free_money;
        // Учитываем деньги от стоп-лосс продаж (приближённо).
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
            // Сколько уже вложено в эту бумагу.
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

            // Сколько можем купить по лимиту веса.
            let count_by_weight = (room / price_f64).floor() as i64;
            // Сколько можем купить на свободные деньги.
            let count_by_money = (free_money / cand.price).to_i64().unwrap_or(0);
            // Ограничение по объёму торгов.
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
