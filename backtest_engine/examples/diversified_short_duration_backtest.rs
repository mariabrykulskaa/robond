//! Бэктест диверсифицированной стратегии коротких облигаций
//!
//! Запуск: cargo run --package backtest_engine --example diversified_short_duration_backtest --release

use backtest_engine::BacktestEngine;
use chrono::NaiveDate;
use history_market_data::MarketDataClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_strategies::diversified_short_duration::DiversifiedShortDurationStrategy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Diversified Short Duration Strategy Backtest ===\n");

    println!("[1/3] Подключаюсь к БД...");
    let client = MarketDataClient::from_env().await?;
    println!("✓ Подключение установлено\n");

    println!("[2/3] Инициализирую движок бэктеста...");
    let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let end_date = NaiveDate::from_ymd_opt(2025, 12, 31).expect("valid date");
    let initial_capital = Decimal::from(1_000_000_i64);

    let engine = BacktestEngine::new(client, initial_capital, start_date, end_date);
    println!("✓ Параметры:");
    println!("  - Начальный капитал: {} руб", initial_capital);
    println!("  - Период: {} — {}\n", start_date, end_date);

    println!("[3/3] Запускаю симуляцию...");
    let strategy = DiversifiedShortDurationStrategy::default();
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

    let json_path = "backtest_diversified.json";
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(json_path, json)?;
    println!("\n✓ Результаты сохранены в {}", json_path);

    Ok(())
}
