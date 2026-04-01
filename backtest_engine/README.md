# Backtest Engine

Модуль для тестирования торговых стратегий на исторических данных облигаций из базы данных PostgreSQL.

## Функционал

- **Чтение исторических данных и информации об облигациях** из модуля `history_market_data`
- **Симуляция торговли**: подсовывание информации об облигациях стратегии, выполнение ордеров
- **Реалистичное исполнение сделок**: используется средняя цена в свечке (середина между low и high)
- **Симуляция выплат**: купоны, амортизации и погашение номинала по датам из БД
- **Симуляция дефолтов**: по данным из БД (type_id 12, 13) и по цене < 20% номинала
- **Обработка оферт**: принудительное погашение облигаций по дате оферты
- **Фильтрация флоатеров**: исключение облигаций с плавающим купоном
- **Last known price**: оценка портфеля в нерабочие дни по последней известной цене

## Компоненты

### MarketSimulator
Основной движок, отслеживающий:
- Текущую дату симуляции
- Портфель инвестора (позиции и кэш)
- Кэшированные цены по датам
- Историю сделок и платежей

### BacktestEngine
Высокоуровневый координатор, который:
- Загружает данные из БД асинхронно
- Управляет временной шкалой симуляции
- Возвращает полный результат с метриками и выписками

### BacktestResult
Итоговый отчёт, содержащий:
- Начальный и финальный капитал
- Прибыль/убыток и % возврата
- Полную историю сделок и платежей
- Снимки портфеля по датам

## Использование

```rust
use backtest_engine::BacktestEngine;
use history_market_data::MarketDataClient;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use trading_strategies::yield_maximizer::YieldMaximizerStrategy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MarketDataClient::from_env().await?;
    let engine = BacktestEngine::new(
        client,
        Decimal::from(1_000_000), // начальный капитал в рублях
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    );
    
    let strategy = YieldMaximizerStrategy::default();
    let result = engine.run_backtest(&strategy).await?;
    
    println!("Начальный капитал: {}", result.initial_capital);
    println!("Финальная стоимость: {:.2}", result.final_value);
    println!("Прибыль/убыток: {:.2}", result.profit_loss);
    println!("Возврат: {:.2}%", result.return_percent);
    
    Ok(())
}
```

## Архитектура

```
BacktestEngine
  ├─ MarketDataClient (читает из БД)
  ├─ MarketSimulator (обработка торговли)
  │   ├─ Portfolio (текущее состояние)
  │   ├─ Price Cache (цены на дату)
  │   └─ Trade/Payment History
  └─ BacktestResult (итоговый отчёт)
```

## Данные из БД

Модуль использует следующие таблицы:
- `bond_bond` — информация об облигациях
- `bond_bondhistorydata` — исторические цены (свечи)
- `bond_payment` — выплаты: купоны, амортизации, погашения, дефолты, оферты
- `bond_coupon` — информация о купонах (размер, периодичность, НКД)

