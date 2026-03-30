use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use trading_strategies::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

use crate::simulator::MarketSimulator;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ─── покупка ────────────────────────────────────────────────────────────────

#[test]
fn test_buy_executes_correctly() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    // open=95, close=96, low=94, high=97, volume=1000, facevalue=1000 руб
    sim.cache_prices("RU000A104H08".to_string(), 95.0, 96.0, 94.0, 97.0, 1000.0, 1000.0, 0.0);

    let order = MarketOrder {
        isin: "RU000A104H08".to_string(),
        order_type: MarketOrderType::Buy,
        count: 10,
    };
    let trade = sim.execute_order(order, true).expect("покупка должна пройти");

    assert_eq!(trade.quantity, 10);
    assert_eq!(trade.side, "buy");
    // mid price = (94+97)/2 = 95.5% от номинала = 955 руб/бумага
    assert!((trade.price - 95.5).abs() < 0.01);
    // итого: 955 * 10 = 9550 руб
    assert!((trade.total_amount - 9_550.0).abs() < 0.01);
    // кэш уменьшился
    assert_eq!(sim.portfolio.free_money, Decimal::from(990_450_i64));
}

// ─── нехватка средств ────────────────────────────────────────────────────────

#[test]
fn test_insufficient_funds_returns_error() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_i64), date(2024, 1, 1));
    sim.cache_prices(
        "RU000A104H08".to_string(),
        98.0,
        99.0,
        97.0,
        100.0,
        50000.0,
        1000.0,
        0.0,
    );

    let order = MarketOrder {
        isin: "RU000A104H08".to_string(),
        order_type: MarketOrderType::Buy,
        count: 100,
    };
    assert!(sim.execute_order(order, true).is_err());
}

// ─── продажа ─────────────────────────────────────────────────────────────────

#[test]
fn test_sell_reduces_holdings_and_returns_cash() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    // Все цены одинаковые → mid = close = 100% → 1000 руб/бумага
    sim.cache_prices("RU0001".to_string(), 100.0, 100.0, 100.0, 100.0, 1000.0, 1000.0, 0.0);

    let buy = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Buy,
        count: 5,
    };
    sim.execute_order(buy, true).unwrap();
    let cash_after_buy = sim.portfolio.free_money;

    let sell = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Sell,
        count: 5,
    };
    sim.execute_order(sell, true).unwrap();

    // всё продано
    assert_eq!(*sim.holdings.get("RU0001").unwrap_or(&0), 0);
    // кэш вернулся
    assert!(sim.portfolio.free_money > cash_after_buy);
}

#[test]
fn test_sell_more_than_held_returns_error() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    sim.cache_prices("RU0001".to_string(), 100.0, 100.0, 100.0, 100.0, 1000.0, 1000.0, 0.0);

    let buy = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Buy,
        count: 3,
    };
    sim.execute_order(buy, true).unwrap();

    let sell = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Sell,
        count: 10,
    };
    assert!(sim.execute_order(sell, true).is_err());
}

// ─── корректность снимка портфеля ─────────────────────────────────────────────

#[test]
fn test_portfolio_snapshot_values_are_correct() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    // close = 100% от номинала 1000 руб → каждая бумага стоит 1000 руб
    sim.cache_prices("RU0001".to_string(), 100.0, 100.0, 100.0, 100.0, 1000.0, 1000.0, 0.0);

    let buy = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Buy,
        count: 10,
    };
    sim.execute_order(buy, true).unwrap();

    let snap = sim.get_portfolio_snapshot();
    let expected_bonds_value = (100.0_f64 / 100.0) * 1000.0 * 10.0; // 10 000 руб
                                                                    // portfolio_value — только бумаги, без кэша
    assert!(
        (snap.portfolio_value - expected_bonds_value).abs() < 0.01,
        "portfolio_value={} ожидалось {}",
        snap.portfolio_value,
        expected_bonds_value
    );
    // total_value = кэш + бумаги = 990 000 + 10 000 = 1 000 000
    assert!(
        (snap.total_value - 1_000_000.0).abs() < 0.01,
        "total_value={} ожидалось 1_000_000",
        snap.total_value
    );
}

// ─── купонные выплаты ─────────────────────────────────────────────────────────

#[test]
fn test_coupon_payment_credited_to_cash() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    // mid price = (94+97)/2 = 95.5% → 955 руб/бумага
    sim.cache_prices("RU0001".to_string(), 95.0, 96.0, 94.0, 97.0, 100.0, 1000.0, 0.0);

    let buy = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Buy,
        count: 10,
    };
    sim.execute_order(buy, true).unwrap();
    let cash_after_buy = sim.portfolio.free_money;

    // Купон 5% от номинала 1000 руб = 50 руб/бумага, 10 бумаг = 500 руб
    let event = sim
        .process_payment("RU0001".to_string(), 5.0, "coupon".to_string())
        .expect("выплата должна состояться");

    assert_eq!(event.quantity, 10);
    assert!(
        (event.amount_per_unit - 50.0).abs() < 0.01,
        "amount_per_unit={}",
        event.amount_per_unit
    );
    assert!(
        (event.total_amount - 500.0).abs() < 0.01,
        "total_amount={}",
        event.total_amount
    );
    // кэш вырос
    assert!(sim.portfolio.free_money > cash_after_buy);
}

// ─── история сделок ───────────────────────────────────────────────────────────

#[test]
fn test_trades_recorded_in_history() {
    let mut sim = MarketSimulator::new(Decimal::from(1_000_000_i64), date(2024, 1, 1));
    sim.cache_prices("RU0001".to_string(), 95.0, 96.0, 94.0, 97.0, 100.0, 1000.0, 0.0);

    let buy = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Buy,
        count: 5,
    };
    sim.execute_order(buy, true).unwrap();

    let sell = MarketOrder {
        isin: "RU0001".to_string(),
        order_type: MarketOrderType::Sell,
        count: 5,
    };
    sim.execute_order(sell, true).unwrap();

    assert_eq!(sim.trades.len(), 2);
    assert_eq!(sim.trades[0].side, "buy");
    assert_eq!(sim.trades[1].side, "sell");
}

// ─── интеграция интерфейса стратегии ─────────────────────────────────────────

/// Стратегия, которая всегда ничего не делает — используется для проверки
/// подписи трейта Strategy без реальной логики.
struct DoNothingStrategy;

impl Strategy for DoNothingStrategy {
    fn decide_trades(
        &self,
        _date: NaiveDate,
        _portfolio: &Portfolio,
        _bonds_info: &HashMap<Isin, BondPersistentInfo>,
        _bonds_prices: &HashMap<Isin, Decimal>,
        _bonds_volumes: &HashMap<Isin, i64>,
    ) -> Vec<MarketOrder> {
        vec![]
    }
}

#[test]
fn test_do_nothing_strategy_returns_no_orders() {
    let strategy = DoNothingStrategy;
    let portfolio = Portfolio {
        free_money: Decimal::from(1_000_000_i64),
        bonds_count: HashMap::new(),
    };
    let orders = strategy.decide_trades(
        date(2024, 1, 1),
        &portfolio,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(orders.is_empty());
}

/// Стратегия-заглушка, которая покупает одну конкретную бумагу если её нет в портфеле.
struct BuyOnceStrategy {
    isin: Isin,
}

impl Strategy for BuyOnceStrategy {
    fn decide_trades(
        &self,
        _date: NaiveDate,
        portfolio: &Portfolio,
        _bonds_info: &HashMap<Isin, BondPersistentInfo>,
        bonds_prices: &HashMap<Isin, Decimal>,
        _bonds_volumes: &HashMap<Isin, i64>,
    ) -> Vec<MarketOrder> {
        // Покупаем только если есть цена и бумаги нет в портфеле
        if bonds_prices.contains_key(&self.isin) && portfolio.bonds_count.get(&self.isin).copied().unwrap_or(0) == 0 {
            vec![MarketOrder {
                isin: self.isin.clone(),
                order_type: MarketOrderType::Buy,
                count: 1,
            }]
        } else {
            vec![]
        }
    }
}

#[test]
fn test_buy_once_strategy_produces_order_when_position_absent() {
    let strategy = BuyOnceStrategy {
        isin: "RU0001".to_string(),
    };
    let portfolio = Portfolio {
        free_money: Decimal::from(1_000_000_i64),
        bonds_count: HashMap::new(),
    };
    let mut prices = HashMap::new();
    prices.insert("RU0001".to_string(), Decimal::from(950_i64));

    let orders = strategy.decide_trades(date(2024, 1, 1), &portfolio, &HashMap::new(), &prices, &HashMap::new());
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].isin, "RU0001");
    assert_eq!(orders[0].order_type, MarketOrderType::Buy);
}

#[test]
fn test_buy_once_strategy_skips_when_position_held() {
    let strategy = BuyOnceStrategy {
        isin: "RU0001".to_string(),
    };
    let mut bonds_count = HashMap::new();
    bonds_count.insert("RU0001".to_string(), 3_i64);
    let portfolio = Portfolio {
        free_money: Decimal::from(1_000_000_i64),
        bonds_count,
    };
    let mut prices = HashMap::new();
    prices.insert("RU0001".to_string(), Decimal::from(950_i64));

    let orders = strategy.decide_trades(date(2024, 1, 1), &portfolio, &HashMap::new(), &prices, &HashMap::new());
    assert!(orders.is_empty());
}
