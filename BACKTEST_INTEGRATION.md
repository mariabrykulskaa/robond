# Интеграция модуля тестирования

## Структура проекта после добавления backtest_engine

```
robond/
├── Cargo.toml (updated: добавлен backtest_engine в workspace)
├── Cargo.lock
├── trading_strategies/        # Стратегии и типы портфеля
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Portfolio, MarketOrder, MarketOrderType
│
├── history_market_data/       # Чтение данных из БД
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs          # MarketDataClient
│       ├── models.rs          # BondHistoryData, BondInfo
│       ├── config.rs
│       └── error.rs
│
├── backtest_engine/           # [НОВЫЙ] Симулятор и бэктест
│   ├── Cargo.toml
│   ├── README.md
│   └── src/
│       ├── lib.rs             # pub use BacktestEngine, MarketSimulator
│       ├── backtest.rs        # BacktestEngine
│       ├── simulator.rs       # MarketSimulator
│       ├── models.rs          # BacktestResult, TradeEvent, PaymentEvent
│       └── tests.rs           # Unit tests
│
└── t-invest-api-rust/         # API интеграция
    └── ...
```

## Workflow использования

### 1. Запуск бэктеста (асинхронно)

```rust
use backtest_engine::BacktestEngine;
use history_market_data::MarketDataClient;
use chrono::NaiveDate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Подключаемся к БД
    let client = MarketDataClient::from_env().await?;
    
    // Создаём движок с начальным капиталом 1 млн руб
    let engine = BacktestEngine::new(
        client,
        1_000_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    );
    
    // Запускаем бэктест
    let result = engine.run_backtest().await?;
    
    // Анализируем результаты
    println!("Начальный капитал: {}", result.initial_capital);
    println!("Финальная стоимость: {:.2}", result.final_value);
    println!("Прибыль/убыток: {:.2}", result.profit_loss);
    println!("Возврат: {:.2}%", result.return_percent);
    
    // Сохраняем результаты в JSON
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write("backtest_result.json", json)?;
    
    Ok(())
}
```

### 2. Интеграция со стратегией (на разработку)

```rust
// Пример для будущей реализации:
struct MyStrategy {
    // поля стратегии
}

impl MyStrategy {
    fn generate_orders(
        &self,
        bond_info: &BondInfo,
        candle: &BondHistoryData,
        portfolio: &Portfolio,
    ) -> Vec<MarketOrder> {
        // Логика стратегии
        vec![]
    }
}
```

### 3. Обработка событий симуляции

В `BacktestEngine::run_backtest()` (на разработку):
- Итерируемся по датам
- Для каждой даты загружаем свечи
- Стратегия генерирует ордеры
- `MarketSimulator::execute_order()` выполняет их
- `MarketSimulator::process_payment()` обрабатывает купоны/погашения
- Сохраняем снимки портфеля

## Ключевые компоненты

### MarketSimulator
- **Отслеживает**: текущее состояние портфеля, цены, историю сделок
- **Выполняет**: покупка/продажа (с проверкой средств), обработка платежей
- **Вычисляет**: стоимость портфеля по рыночным ценам

### BacktestEngine
- **Координирует**: загрузку данных, итерацию по датам
- **Источник данных**: `history_market_data::MarketDataClient`
- **Результат**: полный отчёт `BacktestResult` с метриками и историей

### BacktestResult
- Финальные показатели (капитал, прибыль, %)
- Полная история сделок (`Vec<TradeEvent>`)
- История платежей (`Vec<PaymentEvent>`)
- Снимки портфеля по датам (`Vec<PortfolioSnapshot>`)

## Расширение на предусмотрено

1. **Платежи из БД**: Добавить запросы к `bond_payment` и `bond_coupon`
   ```rust
   async fn get_coupon_payments(
       &self,
       bond_id: i64,
       start: NaiveDate,
       end: NaiveDate,
   ) -> Result<Vec<(NaiveDate, f64)>>
   ```

2. **Стратегии**: Реализовать трейты для различных торговых стратегий

3. **Метрики**: Sharpe ratio, max drawdown, Sortino ratio

4. **Корпоративные действия**: Дефолты, реструктуризация

5. **Комиссии**: Добавить параметры по спредам и комиссиям брокера

## Тестирование

```bash
cd /root/robond
cargo test --package backtest_engine
```

Юнит-тесты покрывают:
- Базовые операции买-sell
- Обработку платежей
- Проверку ошибок (insuffient funds/holdings)
- Оценку портфеля
