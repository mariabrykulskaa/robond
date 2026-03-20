use chrono::NaiveDate;
use history_market_data::{DbConfig, MarketDataClient};

/// Пример использования модуля history_market_data
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Подключение к базе данных из .env...");
    // Рекомендуемый способ: конфиг отдельно, клиент отдельно
    let config = DbConfig::from_env()?;
    let client = MarketDataClient::with_config(&config).await?;
    println!("✓ Подключение установлено");

    // Пример 1: Получить все свечи за определенную дату
    println!("\n=== Пример 1: Получение свечей за дату ===");
    let date = NaiveDate::from_ymd_opt(2025, 6, 11).unwrap();
    let candles = client.get_candles_by_date(date).await?;
    println!("Найдено {} свечей за {}", candles.len(), date);

    if let Some(first_candle) = candles.first() {
        println!("\nПример свечи:");
        println!("  Bond ID: {}", first_candle.bond_id);
        println!("  Дата: {}", first_candle.date);
        println!("  Open: {:?}", first_candle.open);
        println!("  High: {:?}", first_candle.high);
        println!("  Low: {:?}", first_candle.low);
        println!("  Close: {:?}", first_candle.close);
        println!("  Volume: {:?}", first_candle.volume);
        println!("  Num trades: {:?}", first_candle.num_trades);
    }

    // Пример 2: Получить информацию об облигации по ISIN
    println!("\n=== Пример 2: Поиск облигации по ISIN ===");
    let isin = "RU000A10BS76";
    if let Some(bond) = client.get_bond_by_isin(isin).await? {
        println!("Облигация найдена:");
        println!("  ID: {}", bond.id);
        println!("  ISIN: {}", bond.isin.unwrap_or_default());
        println!("  Название: {}", bond.title.unwrap_or_default());
        println!("  Дата погашения: {:?}", bond.maturity_date);
        println!("  Доходность к погашению: {:?}%", bond.yield_to_maturity);
        println!("  Текущая цена: {:?}", bond.price);
        println!("  Торгуется: {}", bond.is_traded);

        // Пример 3: Получить историю для этой облигации
        println!("\n=== Пример 3: История облигации за период ===");
        let start = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
        let history = client.get_bond_candles_range(bond.id, start, end).await?;
        println!("Найдено {} записей за период {} - {}", history.len(), start, end);

        for (i, candle) in history.iter().take(3).enumerate() {
            println!(
                "  {}. {} - Close: {:?}, Volume: {:?}",
                i + 1,
                candle.date,
                candle.close,
                candle.volume
            );
        }
    } else {
        println!("Облигация с ISIN {} не найдена", isin);
    }

    // Пример 4: Получить список торгуемых облигаций
    println!("\n=== Пример 4: Торгуемые облигации ===");
    let traded_bonds = client.get_traded_bonds().await?;
    println!("Всего торгуемых облигаций: {}", traded_bonds.len());

    println!("\nПервые 5 торгуемых облигаций:");
    for (i, bond) in traded_bonds.iter().take(5).enumerate() {
        println!(
            "  {}. {} - {}",
            i + 1,
            bond.isin.as_ref().unwrap_or(&"N/A".to_string()),
            bond.title.as_ref().unwrap_or(&"Без названия".to_string())
        );
    }

    // Пример 5: Получить облигации с пагинацией
    println!("\n=== Пример 5: Пагинация ===");
    let page_size = 10;
    let page = 0;
    let bonds_page = client.get_all_bonds(Some(page_size), Some(page * page_size)).await?;
    println!(
        "Страница {} (размер {}): {} облигаций",
        page,
        page_size,
        bonds_page.len()
    );

    Ok(())
}
