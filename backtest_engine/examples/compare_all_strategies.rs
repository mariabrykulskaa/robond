//! Бэктест всех трёх стратегий для сравнения
//!
//! Запуск: cargo run --package backtest_engine --example compare_all_strategies --release

use backtest_engine::BacktestEngine;
use chrono::{Datelike, NaiveDate, Weekday};
use history_market_data::MarketDataClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_strategies::diversified_short_duration::DiversifiedShortDurationStrategy;
use trading_strategies::high_yield_short::HighYieldShortStrategy;
use trading_strategies::yield_maximizer::YieldMaximizerStrategy;
use trading_strategies::Strategy;

fn compute_metrics(result: &backtest_engine::BacktestResult) -> (f64, f64, f64) {
    let weekday_snapshots: Vec<_> = result
        .portfolio_snapshots
        .iter()
        .filter(|s| !matches!(s.date.weekday(), Weekday::Sat | Weekday::Sun))
        .collect();

    if weekday_snapshots.len() < 2 {
        return (0.0, 0.0, 0.0);
    }

    let daily_returns: Vec<f64> = weekday_snapshots
        .windows(2)
        .map(|w| (w[1].total_value - w[0].total_value) / w[0].total_value)
        .collect();

    let n = daily_returns.len() as f64;
    let mean = daily_returns.iter().sum::<f64>() / n;
    let variance = daily_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();

    let risk_free_daily = (1.0_f64 + 0.18).powf(1.0 / 252.0) - 1.0;
    let sharpe = if std_dev > 1e-12 {
        (mean - risk_free_daily) / std_dev * (252.0_f64).sqrt()
    } else {
        0.0
    };

    // Max drawdown
    let mut peak = weekday_snapshots[0].total_value;
    let mut max_dd = 0.0_f64;
    for s in &weekday_snapshots {
        if s.total_value > peak {
            peak = s.total_value;
        }
        let dd = (peak - s.total_value) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    (sharpe, max_dd * 100.0, mean * 252.0 * 100.0)
}

fn print_result(name: &str, result: &backtest_engine::BacktestResult) {
    let (sharpe, max_dd, annual_ret) = compute_metrics(result);
    let buy_count = result.trades.iter().filter(|t| t.side == "buy").count();
    let sell_count = result.trades.iter().filter(|t| t.side == "sell").count();
    let coupon_total: f64 = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "coupon")
        .map(|p| p.total_amount)
        .sum();

    println!("┌─── {} ───", name);
    println!(
        "│ Финальная стоимость:  {:>12.2} руб",
        result.final_value
    );
    println!(
        "│ Прибыль:              {:>12.2} руб",
        result.profit_loss
    );
    println!(
        "│ Доходность за период: {:>12.2}%",
        result.return_percent
    );
    println!("│ Годовая доходность:   {:>12.2}%", annual_ret);
    println!("│ Sharpe Ratio (rf=18%):{:>12.4}", sharpe);
    println!("│ Макс. просадка:       {:>12.2}%", max_dd);
    println!("│ Покупок: {}, Продаж: {}", buy_count, sell_count);
    println!("│ Купоны получено:      {:>12.2} руб", coupon_total);
    println!("└────────────────────────────────────────");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Сравнение всех стратегий ===\n");

    let client = MarketDataClient::from_env().await?;
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let end_date = NaiveDate::from_ymd_opt(2026, 3, 31).expect("valid date");
    let initial_capital = Decimal::from(1_000_000_i64);

    println!(
        "Период: {} — {}, Капитал: {} руб\n",
        start_date, end_date, initial_capital
    );

    // ── 1. DiversifiedShortDuration ──
    println!("[1/3] DiversifiedShortDurationStrategy...");
    let engine1 = BacktestEngine::new(client.clone(), initial_capital, start_date, end_date);
    let strategy1 = DiversifiedShortDurationStrategy::default();
    let result1 = engine1.run_backtest(&strategy1).await?;
    print_result("DiversifiedShortDuration", &result1);

    let json = serde_json::to_string_pretty(&result1)?;
    std::fs::write("backtest_diversified.json", json)?;
    println!("  → Сохранено в backtest_diversified.json\n");

    // ── 2. HighYieldShort ──
    println!("[2/3] HighYieldShortStrategy...");
    let engine2 = BacktestEngine::new(client.clone(), initial_capital, start_date, end_date);
    let strategy2 = HighYieldShortStrategy::default();
    let result2 = engine2.run_backtest(&strategy2).await?;
    print_result("HighYieldShort", &result2);

    let json = serde_json::to_string_pretty(&result2)?;
    std::fs::write("backtest_highyield.json", json)?;
    println!("  → Сохранено в backtest_highyield.json\n");

    // ── 3. YieldMaximizer ──
    println!("[3/3] YieldMaximizerStrategy...");
    let engine3 = BacktestEngine::new(client.clone(), initial_capital, start_date, end_date);
    let strategy3 = YieldMaximizerStrategy {
        hard_deadline: end_date,
        ..YieldMaximizerStrategy::default()
    };
    let result3 = engine3.run_backtest(&strategy3).await?;
    print_result("YieldMaximizer", &result3);

    let json = serde_json::to_string_pretty(&result3)?;
    std::fs::write("backtest_yieldmax.json", json)?;
    println!("  → Сохранено в backtest_yieldmax.json\n");

    // ── Сводная таблица ──
    println!("=== СВОДНАЯ ТАБЛИЦА ===\n");
    println!(
        "{:<28} {:>12} {:>12} {:>10} {:>10}",
        "Стратегия", "Доходность", "Годовая", "Sharpe", "Макс.DD"
    );
    println!("{}", "─".repeat(76));

    for (name, r) in [
        ("DiversifiedShortDuration", &result1),
        ("HighYieldShort", &result2),
        ("YieldMaximizer", &result3),
    ] {
        let (sharpe, max_dd, annual_ret) = compute_metrics(r);
        println!(
            "{:<28} {:>11.2}% {:>11.2}% {:>10.4} {:>9.2}%",
            name, r.return_percent, annual_ret, sharpe, max_dd
        );
    }

    Ok(())
}
