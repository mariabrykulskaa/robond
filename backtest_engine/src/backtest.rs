//! Главный класс BacktestEngine для запуска полной симуляции

use std::collections::HashMap;

use chrono::NaiveDate;
use history_market_data::MarketDataClient;
use trading_strategies::{BondPersistentInfo, Isin, Money, Strategy};

use crate::models::BacktestResult;
use crate::simulator::MarketSimulator;

/// Основной движок для проведения бэктеста
pub struct BacktestEngine {
    market_data: MarketDataClient,
    initial_capital: Money,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

impl BacktestEngine {
    /// Создаёт новый BacktestEngine
    pub fn new(
        market_data: MarketDataClient,
        initial_capital: Money,
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
    /// 2. Применяет все выплаты (купоны, погашения), запланированные на этот день.
    /// 3. Строит карту текущих цен и передаёт стратегии.
    /// 4. Исполняет ордера, которые вернула стратегия.
    /// 5. Сохраняет снимок портфеля.
    pub async fn run_backtest(&self, strategy: &dyn Strategy) -> Result<BacktestResult, anyhow::Error> {
        // Один раз загружаем всю неизменяемую информацию об облигациях.
        // Пока заполняем только погашение номинала в дату погашения;
        // купонные выплаты появятся, когда будет реализована таблица bond_payment.
        let all_bonds = self.market_data.get_all_bonds(None, None).await?;
        let bonds_info: HashMap<Isin, BondPersistentInfo> = all_bonds
            .into_iter()
            .filter_map(|bond| {
                let isin = bond.isin?;
                let mut payments = Vec::new();
                if let (Some(maturity_date), Some(facevalue)) = (bond.maturity_date, bond.facevalue) {
                    payments.push((maturity_date, facevalue as Money));
                }
                Some((isin, BondPersistentInfo { payments }))
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

            // 2. Применяем выплаты, запланированные на текущий день.
            for (isin, info) in &bonds_info {
                for (payment_date, amount_per_unit) in &info.payments {
                    if *payment_date == current_date {
                        // amount_per_unit хранится в рублях; переводим в % от номинала
                        // для process_payment, которому нужен именно %.
                        if let Some(&facevalue) = simulator.facevalues.get(isin) {
                            let amount_percent = (*amount_per_unit as f64 / facevalue) * 100.0;
                            simulator.process_payment(isin.clone(), amount_percent, "redemption".to_string());
                        }
                    }
                }
            }

            // 3. Строим карту цен (в рублях, целых) для передачи стратегии.
            let bonds_prices: HashMap<Isin, Money> = bonds_info
                .keys()
                .filter_map(|isin| {
                    let key = (current_date, isin.clone());
                    simulator.price_cache.get(&key).map(|&(_, _, low, high, _, facevalue)| {
                        let mid_price_rubles = ((low + high) / 2.0 / 100.0 * facevalue) as Money;
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
        let profit_loss = final_value - self.initial_capital as f64;
        let return_percent = (profit_loss / self.initial_capital as f64) * 100.0;

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
