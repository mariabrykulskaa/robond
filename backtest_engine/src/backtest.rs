//! Главный класс BacktestEngine для запуска полной симуляции

use std::collections::HashMap;

use chrono::NaiveDate;
use history_market_data::MarketDataClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_strategies::{BondCommonInfo, BondPersistentInfo, Isin, PaymentInfo, PaymentType, Strategy};

use crate::models::BacktestResult;
use crate::simulator::MarketSimulator;

/// Основной движок для проведения бэктеста
pub struct BacktestEngine {
    market_data: MarketDataClient,
    initial_capital: Decimal,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl BacktestEngine {
    /// Создаёт новый BacktestEngine
    pub fn new(
        market_data: MarketDataClient,
        initial_capital: Decimal,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        Self {
            market_data,
            initial_capital,
            start_date,
            end_date,
        }
    }

    /// Запускает бэктест с заданной торговой стратегией.
    ///
    /// На каждый торговый день:
    /// 1. Загружает свечи из БД и кэширует цены.
    /// 2. Применяет купоны, амортизации и погашения, запланированные на этот день.
    /// 3. Строит карту текущих цен и передаёт стратегии.
    /// 4. Исполняет ордера, которые вернула стратегия.
    /// 5. Сохраняет снимок портфеля.
    pub async fn run_backtest(&self, strategy: &dyn Strategy) -> Result<BacktestResult, anyhow::Error> {
        // Один раз загружаем список облигаций и строим маппинг bond_id -> ISIN.
        let all_bonds = self.market_data.get_all_bonds(None, None).await?;
        let bond_id_to_isin: HashMap<i64, Isin> =
            all_bonds.iter().filter_map(|b| Some((b.id, b.isin.clone()?))).collect();

        // Загружаем все выплаты за период бэктеста одним запросом.
        // type_id: 1 = амортизация, 2 = купон, 14 = погашение.
        let all_payments = self
            .market_data
            .get_all_bond_payments_in_range(self.start_date, self.end_date)
            .await?;

        // Группируем выплаты по ISIN: (дата, сумма_в_рублях, тип_выплаты).
        let mut bonds_payments: HashMap<Isin, Vec<(NaiveDate, f64, &'static str)>> = HashMap::new();
        for payment in &all_payments {
            let Some(date) = payment.date else { continue };
            let Some(size) = payment.size else { continue };
            if size <= 0.0 {
                continue;
            }
            let Some(bid) = payment.bond_id else { continue };
            let Some(isin) = bond_id_to_isin.get(&bid) else {
                continue;
            };
            let type_name: &'static str = match payment.type_id {
                Some(1) => "amortization",
                Some(2) => "coupon",
                Some(14) => "redemption",
                _ => continue,
            };
            bonds_payments
                .entry(isin.clone())
                .or_default()
                .push((date, size as f64, type_name));
        }

        // Строим bonds_info для стратегии: теперь содержит типизированные выплаты из БД.
        let bonds_info: HashMap<Isin, BondPersistentInfo> = all_bonds
            .iter()
            .filter_map(|bond| {
                let isin = bond.isin.clone()?;
                let payments = bonds_payments
                    .get(&isin)
                    .map(|v| {
                        v.iter()
                            .filter_map(|(date, amount, type_name)| {
                                decimal_from_f64(*amount).map(|d| PaymentInfo {
                                    date: *date,
                                    amount: d,
                                    payment_type: match *type_name {
                                        "coupon" => PaymentType::Coupon,
                                        "amortization" => PaymentType::Amortization,
                                        "redemption" => PaymentType::Redemption,
                                        _ => PaymentType::Coupon,
                                    },
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let bond_info = BondCommonInfo {
                    title: bond.title.clone(),
                    is_subordinated: bond.is_subordinated,
                    issue_volume: bond.issue_volume,
                    placement_date: bond.placement_date,
                    maturity_date: bond.maturity_date,
                    facevalue: bond.facevalue.map(|v| v as f64),
                    start_facevalue: bond.start_facevalue.map(|v| v as f64),
                    board: bond.board.clone(),
                    is_for_qualified_investors: bond.is_for_qualified_investors,
                    is_traded: bond.is_traded,
                };
                Some((isin, BondPersistentInfo { bond_info, payments }))
            })
            .collect();

        let mut simulator = MarketSimulator::new(self.initial_capital, self.start_date);
        let mut snapshots = Vec::new();

        let mut current_date = self.start_date;
        while current_date <= self.end_date {
            simulator.set_date(current_date);

            // 1. Загружаем свечи и кэшируем цены.
            let candles = self.market_data.get_candles_by_date(current_date).await?;
            for candle in candles {
                let bond = self
                    .market_data
                    .get_bond_info(candle.bond_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Bond not found: {}", candle.bond_id))?;

                let isin = bond.isin.unwrap_or_default();
                simulator.cache_prices(
                    isin,
                    candle.open.unwrap_or(0.0),
                    candle.close.unwrap_or(0.0),
                    candle.low.unwrap_or(0.0),
                    candle.high.unwrap_or(0.0),
                    candle.volume.unwrap_or(0.0),
                    candle.facevalue.unwrap_or(100.0),
                );
            }

            // 2. Применяем купоны, амортизации и погашения, запланированные на текущий день.
            // size из bond_payment хранится в рублях; переводим в % от номинала,
            // потому что process_payment ожидает именно процент.
            for (isin, payments) in &bonds_payments {
                for (payment_date, amount_rubles, payment_type) in payments {
                    if *payment_date == current_date {
                        if let Some(&facevalue) = simulator.facevalues.get(isin) {
                            let amount_percent = (amount_rubles / facevalue) * 100.0;
                            simulator.process_payment(isin.clone(), amount_percent, payment_type.to_string());
                        }
                    }
                }
            }

            // 3. Строим карту цен (в рублях, целых) для передачи стратегии.
            let bonds_prices: HashMap<Isin, Decimal> = bonds_info
                .keys()
                .filter_map(|isin| {
                    let key = (current_date, isin.clone());
                    simulator.price_cache.get(&key).map(|&(_, _, low, high, _, facevalue)| {
                        let mid_price_rubles = decimal_from_f64((low + high) / 2.0 / 100.0 * facevalue)
                            .unwrap_or_else(|| Decimal::ZERO);
                        (isin.clone(), mid_price_rubles)
                    })
                })
                .collect();

            // 4. Вызываем стратегию и исполняем ордера.
            let portfolio = simulator.portfolio.clone();
            let orders = strategy.decide_trades(current_date, &portfolio, &bonds_info, &bonds_prices);
            for order in orders {
                if let Err(e) = simulator.execute_order(order, true) {
                    eprintln!("Ошибка исполнения ордера на {}: {}", current_date, e);
                }
            }

            // 5. Снимок портфеля на конец дня.
            snapshots.push(simulator.get_portfolio_snapshot());

            current_date += chrono::Duration::days(1);
        }

        let final_value = simulator.get_portfolio_value();
        let initial_capital_f64 = self.initial_capital.to_f64().unwrap_or(0.0);
        let profit_loss = final_value - initial_capital_f64;
        let return_percent = if initial_capital_f64.abs() > f64::EPSILON {
            (profit_loss / initial_capital_f64) * 100.0
        } else {
            0.0
        };

        Ok(BacktestResult {
            initial_capital: self.initial_capital,
            final_value,
            profit_loss,
            return_percent,
            trades: simulator.trades,
            payments: simulator.payments,
            portfolio_snapshots: snapshots,
            start_date: self.start_date,
            end_date: self.end_date,
        })
    }
}

fn decimal_from_f64(value: f64) -> Option<Decimal> {
    value.to_string().parse::<Decimal>().ok()
}
