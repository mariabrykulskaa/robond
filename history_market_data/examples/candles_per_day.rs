//! Подсчёт свечей по датам за заданный месяц.
//!
//! Использование:
//!   cargo run --example candles_per_day -- 2025 6
//!
//! Результат записывается в файл `candles_YYYY_MM.txt`.

use std::collections::BTreeMap;
use std::env;
use std::io::Write;

use chrono::NaiveDate;
use history_market_data::{DbConfig, MarketDataClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- аргументы: год и месяц ---
    let args: Vec<String> = env::args().collect();
    let (year, month) = match args.as_slice() {
        [_, y, m] => (y.parse::<i32>()?, m.parse::<u32>()?),
        _ => {
            eprintln!("Использование: cargo run --example candles_per_day -- <год> <месяц>");
            eprintln!("Пример:        cargo run --example candles_per_day -- 2025 6");
            std::process::exit(1);
        }
    };

    // --- подключение ---
    let config = DbConfig::from_env()?;
    let client = MarketDataClient::with_config(&config).await?;
    println!("Подключено. Получаем данные за {year}-{month:02}...");

    // --- диапазон дат месяца ---
    let start = NaiveDate::from_ymd_opt(year, month, 1).expect("некорректный месяц");
    let end = last_day_of_month(year, month);

    // --- загрузка всех свечей за месяц одним запросом ---
    // Используем get_candles_by_date для каждой даты или
    // один запрос напрямую через sqlx (чтобы не делать N запросов).
    // Поскольку публичный API даёт get_candles_by_date(date),
    // воспользуемся им в цикле — для монолита это достаточно.
    let mut counts: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    let mut current = start;
    while current <= end {
        let candles = client.get_candles_by_date(current).await?;
        counts.insert(current, candles.len());
        current = current.succ_opt().expect("дата за пределами диапазона");
    }

    // --- вывод в файл ---
    let filename = format!("candles_{year}_{month:02}.txt");
    let mut file = std::fs::File::create(&filename)?;

    writeln!(file, "Свечи по датам за {year}-{month:02}")?;
    writeln!(file, "{:-<30}", "")?;

    let mut total = 0usize;
    for (date, count) in &counts {
        writeln!(file, "{date}  {count:>6} свечей")?;
        total += count;
    }

    writeln!(file, "{:-<30}", "")?;
    writeln!(file, "Итого за месяц: {total} свечей")?;

    println!("Готово! Результат в файле `{filename}`");
    println!("Всего свечей за {year}-{month:02}: {total}");

    Ok(())
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    // первый день следующего месяца минус 1 день
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}
