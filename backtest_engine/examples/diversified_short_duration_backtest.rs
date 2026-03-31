//! Бэктест диверсифицированной стратегии коротких облигаций
//!
//! Запуск: cargo run --package backtest_engine --example diversified_short_duration_backtest --release

use backtest_engine::BacktestEngine;
use chrono::{Datelike, NaiveDate, Weekday};
use history_market_data::MarketDataClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_strategies::yield_maximizer::YieldMaximizerStrategy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Diversified Short Duration Strategy Backtest ===\n");

    println!("[1/3] Подключаюсь к БД...");
    let client = MarketDataClient::from_env().await?;
    println!("✓ Подключение установлено\n");

    println!("[2/3] Инициализирую движок бэктеста...");
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let end_date = NaiveDate::from_ymd_opt(2026, 3, 31).expect("valid date");
    let initial_capital = Decimal::from(1_000_000_i64);

    let engine = BacktestEngine::new(client, initial_capital, start_date, end_date);
    println!("✓ Параметры:");
    println!("  - Начальный капитал: {} руб", initial_capital);
    println!("  - Период: {} — {}\n", start_date, end_date);

    println!("[3/3] Запускаю симуляцию...");
    let strategy = YieldMaximizerStrategy {
        hard_deadline: end_date,
        ..YieldMaximizerStrategy::default()
    };
    println!("  Параметры стратегии:");
    println!("    - Мин. XIRR для покупки: {:.0}%", strategy.min_yield * 100.0);
    println!("    - Макс. XIRR (distress): {:.0}%", strategy.max_yield * 100.0);
    println!("    - Мин. XIRR (urgent):    {:.0}%", strategy.min_yield_urgent * 100.0);
    println!("    - Порог urgent кэша:     {:.0}%", strategy.cash_urgency_threshold * 100.0);
    println!("    - Макс. вес позиции:     {:.0}%", strategy.max_weight * 100.0);
    println!("    - Мин. объём для покупки: {} лотов", strategy.min_volume_for_buy);
    println!("    - Hard deadline:         {}", strategy.hard_deadline);
    println!("    - Стоп-лосс:             НЕТ (hold to maturity)");
    let result = engine.run_backtest(&strategy).await?;
    println!("✓ Симуляция завершена\n");

    println!("=== РЕЗУЛЬТАТЫ ===\n");
    println!(
        "Начальный капитал:    {:>15.2} руб",
        result.initial_capital.to_f64().unwrap_or(0.0)
    );
    println!("Финальная стоимость:  {:>15.2} руб", result.final_value);
    println!("Прибыль/Убыток:       {:>15.2} руб", result.profit_loss);
    println!("Возврат:              {:>15.2} %", result.return_percent);

    println!("\nСтатистика сделок:");
    let buy_count = result.trades.iter().filter(|t| t.side == "buy").count();
    let sell_count = result.trades.iter().filter(|t| t.side == "sell").count();
    println!("  Покупок:  {}", buy_count);
    println!("  Продаж:   {}", sell_count);

    println!("\nСтатистика платежей:");
    let coupon_total: f64 = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "coupon")
        .map(|p| p.total_amount)
        .sum();
    let amort_total: f64 = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "amortization")
        .map(|p| p.total_amount)
        .sum();
    let redemption_total: f64 = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "redemption")
        .map(|p| p.total_amount)
        .sum();
    println!("  Купоны:     {:.2} руб", coupon_total);
    println!("  Амортизации:{:.2} руб", amort_total);
    println!("  Погашения:  {:.2} руб", redemption_total);

    if let Some(first) = result.portfolio_snapshots.first() {
        println!("\nПервый снимок ({}): {:.2} руб", first.date, first.total_value);
    }
    if let Some(last) = result.portfolio_snapshots.last() {
        println!("Последний    ({}): {:.2} руб", last.date, last.total_value);
    }

    // Sharpe ratio (без выходных)
    let weekday_snapshots: Vec<_> = result
        .portfolio_snapshots
        .iter()
        .filter(|s| !matches!(s.date.weekday(), Weekday::Sat | Weekday::Sun))
        .collect();

    if weekday_snapshots.len() >= 2 {
        let daily_returns: Vec<f64> = weekday_snapshots
            .windows(2)
            .map(|w| (w[1].total_value - w[0].total_value) / w[0].total_value)
            .collect();

        let n = daily_returns.len() as f64;
        let mean = daily_returns.iter().sum::<f64>() / n;
        let variance = daily_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();

        let risk_free_daily = (1.0_f64 + 0.18).powf(1.0 / 252.0) - 1.0; // rf = 18%
        let sharpe = if std_dev > 1e-12 {
            (mean - risk_free_daily) / std_dev * (252.0_f64).sqrt()
        } else {
            0.0
        };

        println!("\nРиск-метрики (только рабочие дни, {} точек):", weekday_snapshots.len());
        println!("  Средняя дневная доходность: {:.6}%", mean * 100.0);
        println!("  Стд. откл. дневной дох.:    {:.6}%", std_dev * 100.0);
        println!("  Годовая доходность:         {:.2}%", mean * 252.0 * 100.0);
        println!("  Годовое стд. откл.:         {:.2}%", std_dev * (252.0_f64).sqrt() * 100.0);
        println!("  Sharpe Ratio (rf=18%):       {:.4}", sharpe);
    }

    let json_path = "backtest_diversified.json";
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(json_path, json)?;
    println!("\n✓ Результаты сохранены в {}", json_path);

    Ok(())
}
