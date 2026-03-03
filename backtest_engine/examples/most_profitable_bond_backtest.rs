//! Пример использования backtest_engine
//!
//! Запуск: cargo run --package backtest_engine --example most_profitable_bond_backtest --release

use std::collections::HashMap;

use backtest_engine::BacktestEngine;
use chrono::NaiveDate;
use history_market_data::MarketDataClient;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use trading_strategies::{
    strategies::MostProfitableBondStrategy, BondPersistentInfo, Isin, MarketOrder, Portfolio, Strategy,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Backtest Engine Example ===\n");

    // Подключаемся к БД через .env
    println!("[1/3] Подключаюсь к БД...");
    let client = MarketDataClient::from_env().await?;
    println!("✓ Подключение установлено\n");

    // Создаём движок бэктеста с диапазоном дат
    println!("[2/3] Инициализирую движок бэктеста...");
    let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date");
    let end_date = NaiveDate::from_ymd_opt(2025, 11, 1).expect("valid date");
    let initial_capital = Decimal::from(1_000_000_i64);

    let engine = BacktestEngine::new(client, initial_capital, start_date, end_date);
    println!("✓ Параметры движка:");
    println!("  - Начальный капитал: {} руб", initial_capital);
    println!("  - Период: {} до {}\n", start_date, end_date);

    // Запускаем симуляцию
    println!("[3/3] Запускаю симуляцию историческихданных...");
    let result = engine.run_backtest(&MostProfitableBondStrategy).await?;
    println!("✓ Симуляция завершена\n");

    // Выводим результаты
    println!("=== РЕЗУЛЬТАТЫ БЭКТЕСТА ===\n");
    println!(
        "Начальный капитал:    {:>15.2} руб",
        result.initial_capital.to_f64().unwrap_or(0.0)
    );
    println!("Финальная стоимость:  {:>15.2} руб", result.final_value);
    println!("Прибыль/Убыток:       {:>15.2} руб", result.profit_loss);
    println!("Возврат:              {:>15.2} %", result.return_percent);
    println!("\n---\n");

    // Выводим статистику сделок
    println!("Статистика сделок:");
    println!("  Всего сделок:  {}", result.trades.len());
    let buy_count = result.trades.iter().filter(|t| t.side == "buy").count();
    let sell_count = result.trades.iter().filter(|t| t.side == "sell").count();
    println!("  Покупок:       {}", buy_count);
    println!("  Продаж:        {}", sell_count);

    let total_traded = result.trades.iter().map(|t| t.total_amount).sum::<f64>();
    println!("  Сумма торгов:  {:.2} руб", total_traded);

    // Выводим результатпо платежам
    println!("\nСтатистика платежей:");
    println!("  Всего платежей: {}", result.payments.len());
    let coupon_payments = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "coupon")
        .map(|p| p.total_amount)
        .sum::<f64>();
    let redemption_payments = result
        .payments
        .iter()
        .filter(|p| p.payment_type == "redemption")
        .map(|p| p.total_amount)
        .sum::<f64>();

    if coupon_payments > 0.0 {
        println!("  Купоны:        {:.2} руб", coupon_payments);
    }
    if redemption_payments > 0.0 {
        println!("  Погашения:     {:.2} руб", redemption_payments);
    }

    // Снимки портфеля
    println!("\nПортфель ({} снимков):", result.portfolio_snapshots.len());
    if let Some(first) = result.portfolio_snapshots.first() {
        println!("  Начало ({}): {} руб", first.date, first.total_value);
    }
    if let Some(last) = result.portfolio_snapshots.last() {
        println!("  Конец   ({}): {:.2} руб", last.date, last.total_value);
    }

    // Сохраняем результ в JSON (если нужно)
    let json_path = "backtest_result.json";
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(json_path, json)?;
    println!("\n✓ Полные результаты сохранены в {}", json_path);

    Ok(())
}
