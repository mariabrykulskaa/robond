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
        use std::time::Instant;
        let t0 = Instant::now();

        // Один раз загружаем список облигаций и строим маппинг bond_id -> ISIN.
        eprintln!("  Загрузка списка облигаций из БД...");
        let all_bonds = self.market_data.get_all_bonds(None, None).await?;
        let bond_id_to_isin: HashMap<i64, Isin> =
            all_bonds.iter().filter_map(|b| Some((b.id, b.isin.clone()?))).collect();
        eprintln!(
            "  ✓ Загружено {} облигаций ({:.1}с)",
            all_bonds.len(),
            t0.elapsed().as_secs_f64()
        );

        // Загружаем даты оферт — облигации с офертой обрабатываем до даты оферты.
        eprintln!("  Загрузка дат оферт...");
        let t_offers = Instant::now();
        let bond_offer_dates = self.market_data.get_bond_offer_dates().await?;
        eprintln!(
            "  ✓ {} облигаций с офертой ({:.1}с)",
            bond_offer_dates.len(),
            t_offers.elapsed().as_secs_f64()
        );

        // Загружаем купонную информацию для всех облигаций одним запросом.
        eprintln!("  Загрузка купонной информации...");
        let t_coupons = Instant::now();
        let bond_coupons = self.market_data.get_all_bond_coupons().await?;
        eprintln!(
            "  ✓ Купоны для {} облигаций ({:.1}с)",
            bond_coupons.len(),
            t_coupons.elapsed().as_secs_f64()
        );
        // Загружаем даты дефолтов (type_id 12,13 — дефолт/тех.дефолт по оферте).
        eprintln!("  Загрузка дат дефолтов...");
        let t_defaults = Instant::now();
        let bond_default_dates = self.market_data.get_bond_default_dates().await?;
        eprintln!(
            "  ✓ {} облигаций с дефолтом ({:.1}с)",
            bond_default_dates.len(),
            t_defaults.elapsed().as_secs_f64()
        );
        // Загружаем все свечи за весь период одним запросом (без JSON — экономия памяти).
        eprintln!(
            "  Загрузка всех свечей за период {}..{} ...",
            self.start_date, self.end_date
        );
        let t_candles = Instant::now();
        let all_candles = self
            .market_data
            .get_all_candles_in_range(self.start_date, self.end_date)
            .await?;
        eprintln!(
            "  ✓ Загружено {} свечей ({:.1}с)",
            all_candles.len(),
            t_candles.elapsed().as_secs_f64()
        );

        // Индексируем свечи по дате для O(1) доступа в дневном цикле.
        // Используем owned-данные, чтобы можно было drop(all_candles) и освободить дублирующую память.
        let mut candles_by_date: HashMap<NaiveDate, Vec<history_market_data::BondHistoryData>> = HashMap::new();
        for candle in all_candles {
            candles_by_date.entry(candle.date).or_default().push(candle);
        }
        // all_candles moved — память освобождена.

        // Загружаем все выплаты за период бэктеста одним запросом.
        // type_id: 1 = амортизация, 2 = купон, 14 = погашение.
        eprintln!("  Загрузка выплат за период {}..{} ...", self.start_date, self.end_date);
        let t1 = Instant::now();
        let all_payments = self
            .market_data
            .get_all_bond_payments_in_range(self.start_date, self.end_date)
            .await?;
        eprintln!(
            "  ✓ Загружено {} выплат ({:.1}с)",
            all_payments.len(),
            t1.elapsed().as_secs_f64()
        );

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

        // ID рублёвой валюты (SUR) в таблице bond_currency.
        const RUB_CURRENCY_ID: i64 = 3;

        // Доска структурных (инвестиционных) облигаций на MOEX — исключаем из бэктеста.
        const STRUCTURAL_BOARD: &str = "TQIR";

        // Строим bonds_info для стратегии: только рублёвые, не структурные, не флоатеры.
        eprintln!("  Построение карты облигаций и индексов...");
        let t2 = Instant::now();
        let bonds_info: HashMap<Isin, BondPersistentInfo> = all_bonds
            .iter()
            .filter(|bond| bond.currency_id == Some(RUB_CURRENCY_ID))
            .filter(|bond| bond.board.as_deref() != Some(STRUCTURAL_BOARD))
            .filter(|bond| {
                // Исключаем флоатеры: если купоны отличаются друг от друга.
                let dominated = bond.isin.as_ref().and_then(|isin| bonds_payments.get(isin));
                !dominated.map(|p| is_floater(p)).unwrap_or(false)
            })
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
                    isin: isin.clone(),
                    currency_id: bond.currency_id,
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
                    offer_date: bond_offer_dates.get(&bond.id).copied(),
                    default_date: bond_default_dates.get(&bond.id).copied(),
                    coupon_size: bond_coupons.get(&bond.id).and_then(|c| c.size.map(|v| v as f64)),
                    coupon_period: bond_coupons.get(&bond.id).and_then(|c| c.period),
                    coupon_aci: bond_coupons.get(&bond.id).and_then(|c| c.aci.map(|v| v as f64)),
                };
                Some((isin, BondPersistentInfo { bond_info, payments }))
            })
            .collect();

        // Маппинг bond_id -> ISIN для быстрого разрешения свечей без запросов к БД.
        // (bond_id_to_isin уже построен выше)

        // Индексируем выплаты по дате для O(1) доступа в дневном цикле.
        let mut payments_by_date: HashMap<NaiveDate, Vec<(Isin, f64, &'static str)>> = HashMap::new();
        for (isin, payments) in &bonds_payments {
            for &(date, amount, ptype) in payments {
                payments_by_date
                    .entry(date)
                    .or_default()
                    .push((isin.clone(), amount, ptype));
            }
        }

        eprintln!(
            "  ✓ {} рублёвых облигаций, выплаты проиндексированы ({:.1}с)",
            bonds_info.len(),
            t2.elapsed().as_secs_f64()
        );

        // Индекс дат офертных погашений: дата -> список ISIN.
        let mut offer_isins_by_date: HashMap<NaiveDate, Vec<Isin>> = HashMap::new();
        for (isin, info) in &bonds_info {
            if let Some(offer_date) = info.bond_info.offer_date {
                offer_isins_by_date.entry(offer_date).or_default().push(isin.clone());
            }
        }

        // Индекс дат дефолтов: дата -> список ISIN.
        let mut default_isins_by_date: HashMap<NaiveDate, Vec<Isin>> = HashMap::new();
        for (isin, info) in &bonds_info {
            if let Some(default_date) = info.bond_info.default_date {
                default_isins_by_date
                    .entry(default_date)
                    .or_default()
                    .push(isin.clone());
            }
        }

        // Множество дефолтных ISIN — накапливается по мере прохождения дат.
        // Облигации из этого множества исключаются из торгов и выплат.
        let mut defaulted_isins: std::collections::HashSet<Isin> = std::collections::HashSet::new();

        eprintln!(
            "  Инициализация завершена за {:.1}с. Запуск цикла по датам...\n",
            t0.elapsed().as_secs_f64()
        );

        let mut simulator = MarketSimulator::new(self.initial_capital, self.start_date);

        // Заполняем объёмы выпуска для ограничения покупок.
        for (isin, info) in &bonds_info {
            if let Some(vol) = info.bond_info.issue_volume {
                simulator.issue_volumes.insert(isin.clone(), vol);
            }
        }

        let mut snapshots = Vec::new();

        let total_days = (self.end_date - self.start_date).num_days() + 1;
        let mut current_date = self.start_date;
        let mut day_num: i64 = 0;
        while current_date <= self.end_date {
            day_num += 1;
            eprintln!("[{}/{}] {} ...", day_num, total_days, current_date);
            simulator.set_date(current_date);

            // 1. Кэшируем цены из предзагруженных свечей.
            if let Some(candles) = candles_by_date.remove(&current_date) {
                for candle in &candles {
                    let Some(isin) = bond_id_to_isin.get(&candle.bond_id) else {
                        continue;
                    };
                    simulator.cache_prices(
                        isin.clone(),
                        candle.open.unwrap_or(0.0),
                        candle.close.unwrap_or(0.0),
                        candle.low.unwrap_or(0.0),
                        candle.high.unwrap_or(0.0),
                        candle.volume.unwrap_or(0.0),
                        candle.facevalue.unwrap_or(100.0),
                        candle.accint.unwrap_or(0.0),
                    );
                }
            }

            // 1.5. Обработка дефолтов.
            //   a) По дате из БД (тип платежа 12/13).
            if let Some(def_isins) = default_isins_by_date.get(&current_date) {
                for isin in def_isins {
                    if !defaulted_isins.contains(isin) {
                        defaulted_isins.insert(isin.clone());
                        if let Some(event) = simulator.write_off_bond(isin) {
                            eprintln!("  Дефолт (БД): {} — списано {} шт.", isin, event.quantity);
                        }
                    }
                }
            }
            //   b) По цене < 20% номинала — считаем облигацию дефолтной.
            let mut new_defaults = Vec::new();
            if let Some(today_isins) = simulator.isins_by_date.get(&current_date) {
                for isin in today_isins {
                    if defaulted_isins.contains(isin) {
                        continue;
                    }
                    let key = (current_date, isin.clone());
                    if let Some(&(_, _, low, high, _, _facevalue, _)) = simulator.price_cache.get(&key) {
                        let mid_price_percent = (low + high) / 2.0;
                        if mid_price_percent > 0.0 && mid_price_percent < 20.0 {
                            new_defaults.push((isin.clone(), mid_price_percent));
                        }
                    }
                }
            }
            for (isin, mid_price_percent) in new_defaults {
                defaulted_isins.insert(isin.clone());
                if let Some(event) = simulator.write_off_bond(&isin) {
                    eprintln!(
                        "  Дефолт (цена {:.1}% < 20%): {} — списано {} шт.",
                        mid_price_percent, isin, event.quantity
                    );
                }
            }

            // 2. Принудительное погашение облигаций по оферте (выкуп по номиналу).
            if let Some(offer_isins) = offer_isins_by_date.get(&current_date) {
                for isin in offer_isins {
                    if let Some(event) = simulator.force_redeem_bond(isin) {
                        eprintln!(
                            "  Оферта: {} — погашено {} шт. на {:.2} руб.",
                            isin, event.quantity, event.total_amount
                        );
                    }
                }
            }

            // 3. Применяем купоны, амортизации и погашения, запланированные на текущий день.
            // size из bond_payment хранится в рублях; переводим в % от номинала,
            // потому что process_payment ожидает именно процент.
            if let Some(today_payments) = payments_by_date.get(&current_date) {
                for (isin, amount_rubles, payment_type) in today_payments {
                    if defaulted_isins.contains(isin) {
                        continue;
                    }
                    if let Some(&facevalue) = simulator.facevalues.get(isin) {
                        let amount_percent = (amount_rubles / facevalue) * 100.0;
                        simulator.process_payment(isin.clone(), amount_percent, payment_type.to_string());
                    }
                }
            }

            // 4. Строим карту цен (в рублях, целых) для передачи стратегии.
            //    Включаем НКД в цену (грязная цена).
            //    Если свечи на текущий день нет — берём последнюю известную цену (last_known_price),
            //    чтобы отсутствие торгов не обнуляло оценку позиции.
            let bonds_prices: HashMap<Isin, Decimal> = bonds_info
                .keys()
                .filter(|isin| !defaulted_isins.contains(*isin))
                .filter_map(|isin| {
                    let key = (current_date, isin.clone());
                    let entry = simulator
                        .price_cache
                        .get(&key)
                        .or_else(|| simulator.last_known_price.get(isin.as_str()));
                    entry.filter(|&&(_, _, _, _, volume, _, _)| volume > 0.0).map(
                        |&(_, _, low, high, _, facevalue, accint)| {
                            let mid_price_rubles = decimal_from_f64((low + high) / 2.0 / 100.0 * facevalue + accint)
                                .unwrap_or(Decimal::ZERO);
                            (isin.clone(), mid_price_rubles)
                        },
                    )
                })
                .collect();

            // 5. Вызываем стратегию и исполняем ордера.
            //    Объёмы торгов: для last_known_price ставим 0 (торговать нельзя, только оценка).
            let bonds_volumes: HashMap<Isin, i64> = bonds_info
                .keys()
                .filter(|isin| !defaulted_isins.contains(*isin))
                .filter_map(|isin| {
                    let key = (current_date, isin.clone());
                    if let Some(&(_, _, _, _, volume, _, _)) = simulator.price_cache.get(&key) {
                        Some((isin.clone(), volume as i64))
                    } else if simulator.last_known_price.contains_key(isin.as_str()) {
                        // Есть last_known_price, но сегодня нет торгов — объём = 0.
                        Some((isin.clone(), 0))
                    } else {
                        None
                    }
                })
                .collect();
            let portfolio = simulator.portfolio.clone();
            let orders = strategy.decide_trades(current_date, &portfolio, &bonds_info, &bonds_prices, &bonds_volumes);
            for order in orders {
                if let Err(e) = simulator.execute_order(order, true) {
                    eprintln!("Ошибка исполнения ордера на {}: {}", current_date, e);
                }
            }

            // 6. Снимок портфеля на конец дня.
            snapshots.push(simulator.get_portfolio_snapshot());

            // 7. Очищаем price_cache и isins_by_date за текущий день — данные уже в last_known_price.
            //    Это не даёт кэши расти бесконечно и экономит RAM.
            simulator.price_cache.retain(|(d, _), _| *d != current_date);
            simulator.isins_by_date.remove(&current_date);

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

/// Определяет, является ли облигация флоатером (плавающий купон).
///
/// Сравнивает размеры купонных выплат: если хотя бы два купона
/// отличаются друг от друга — облигация считается флоатером.
fn is_floater(payments: &[(NaiveDate, f64, &str)]) -> bool {
    let coupon_sizes: Vec<f64> = payments
        .iter()
        .filter(|(_, _, t)| *t == "coupon")
        .map(|(_, amount, _)| *amount)
        .collect();
    if coupon_sizes.len() < 2 {
        return false;
    }
    let first = coupon_sizes[0];
    coupon_sizes.iter().any(|&s| (s - first).abs() > 0.01)
}

/// Построить карту `bonds_info` из загруженных данных.
///
/// Фильтрует только рублёвые облигации (currency_id = 3) и исключает
/// структурные (доска TQIR). Для каждой облигации собирает платежи,
/// оферты, дефолты и купонную информацию.
///
/// Используется движком бэктеста, но может быть вызвана отдельно —
/// например, для анализа портфеля или скоринга облигаций.
pub async fn build_bonds_info(
    market_data: &MarketDataClient,
) -> Result<HashMap<Isin, BondPersistentInfo>, anyhow::Error> {
    // ID рублёвой валюты (SUR) в таблице bond_currency.
    const RUB_CURRENCY_ID: i64 = 3;
    // Доска структурных (инвестиционных) облигаций на MOEX — исключаем.
    const STRUCTURAL_BOARD: &str = "TQIR";

    let all_bonds = market_data.get_all_bonds(None, None).await?;
    let bond_id_to_isin: HashMap<i64, Isin> = all_bonds.iter().filter_map(|b| Some((b.id, b.isin.clone()?))).collect();

    let bond_offer_dates = market_data.get_bond_offer_dates().await?;
    let bond_default_dates = market_data.get_bond_default_dates().await?;
    let bond_coupons = market_data.get_all_bond_coupons().await?;

    // Загружаем ВСЕ выплаты (без ограничения по дате).
    let all_payments = market_data
        .get_all_bond_payments_in_range(
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
        )
        .await?;

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

    let bonds_info: HashMap<Isin, BondPersistentInfo> = all_bonds
        .iter()
        .filter(|bond| bond.currency_id == Some(RUB_CURRENCY_ID))
        .filter(|bond| bond.board.as_deref() != Some(STRUCTURAL_BOARD))
        .filter(|bond| {
            let dominated = bond.isin.as_ref().and_then(|isin| bonds_payments.get(isin));
            !dominated.map(|p| is_floater(p)).unwrap_or(false)
        })
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
                isin: isin.clone(),
                currency_id: bond.currency_id,
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
                offer_date: bond_offer_dates.get(&bond.id).copied(),
                default_date: bond_default_dates.get(&bond.id).copied(),
                coupon_size: bond_coupons.get(&bond.id).and_then(|c| c.size.map(|v| v as f64)),
                coupon_period: bond_coupons.get(&bond.id).and_then(|c| c.period),
                coupon_aci: bond_coupons.get(&bond.id).and_then(|c| c.aci.map(|v| v as f64)),
            };
            Some((isin, BondPersistentInfo { bond_info, payments }))
        })
        .collect();

    Ok(bonds_info)
}
