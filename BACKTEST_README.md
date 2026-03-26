# Модуль тестирования робота на исторических данных

Новый модуль **`backtest_engine`** для тестирования торговых стратегий на исторических данных облигаций. ✓ Готов к использованию.

## Быстрый старт

```bash
# В корневой папке robond:
cd /root/robond

# Собрать модуль
cargo build --package backtest_engine

# Запустить тесты
cargo test --package backtest_engine

# Запустить пример
cargo run --package backtest_engine --example simple_backtest
```

## Что реализовано

### ✓ Чтение информации об облигациях из БД
Интеграция с модулем `history_market_data`:
- `get_bond_info()` - информация об облигации
- `get_candles_by_date()` - исторические цены
- `get_bond_candles_range()` - цены за период
- `get_all_bonds()` - список облигаций

### ✓ Симуляция сделок
- Покупка/продажа облигаций
- Использование средней цены свечи (mid-price)
- Проверка валидности (достаточно средств, позиций)
- История всех сделок с деталями

### ✓ Обработка платежей (базовая)
- Купоны (coupon)
- Погашение номинала (redemption)
- Расчёт размера выплаты на основе номинала

### ✓ Портфель и метрики
- Отслеживание позиций
- Расчёт стоимости портфеля
- Начальный и финальный капитал
- Прибыль/убыток и % возврата
- Снимки портфеля по датам

## Структура модуля

```
backtest_engine/
├── BacktestEngine          # Главный движок координации
│   ├── run_backtest()      # Запуск полной симуляции
│   └── load historical data from DB
│
├── MarketSimulator         # Движок рынка и портфеля
│   ├── execute_order()     # Обработка ордеров
│   ├── process_payment()   # Обработка платежей
│   ├── cache_prices()      # Кеширование цен
│   └── get_portfolio_value() # Оценка портфеля
│
└── BacktestResult          # Итоговый отчёт
    ├── trades              # История сделок
    ├── payments            # История платежей
    ├── snapshots           # Снимки портфеля
    └── metrics             # Финальные показатели
```

## Пример использования

```rust
use backtest_engine::BacktestEngine;
use history_market_data::MarketDataClient;
use chrono::NaiveDate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Подключаемся к БД
    let client = MarketDataClient::from_env().await?;
    
    // Создаём движок
    let engine = BacktestEngine::new(
        client,
        1_000_000,  // начальный капитал
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    );
    
    // Запускаем симуляцию
    let result = engine.run_backtest().await?;
    
    // Анализируем результаты
    println!("Начальный: {}", result.initial_capital);
    println!("Финальный: {:.2}", result.final_value);
    println!("Прибыль:   {:.2}", result.profit_loss);
    println!("Возврат:   {:.2}%", result.return_percent);
    
    Ok(())
}
```

## Тесты

Модуль включает unit-тесты для:
- ✓ Базовых операций buy/sell
- ✓ Обработки платежей
- ✓ Проверки валидности (недостаточно средств, позиций)
- ✓ Оценки портфеля

Запуск: `cargo test --package backtest_engine`

## Что ещё нужно добавить

### Phase 2: Платежи из БД (на разработку)
- Запросы к `bond_payment` для получения фактических купонов/погашений
- Добавить методы в `MarketDataClient::get_bond_payments()`

### Phase 3: Интеграция стратегий (на разработку)
- Трейт `TradingStrategy`
- Цикл: стратегия → ордеры → симулятор
- Примеры готовых стратегий

### Phase 4: Продвинутые метрики (на разработку)
- Sharpe Ratio
- Maximum Drawdown
- Win Rate
- Profit Factor

### Phase 5: Корпоративные действия (на разработку)
- Дефолты облигаций
- Переиндексирование номиналов
- Срочные события

## Файлы проекта

```
backtest_engine/
├── Cargo.toml
├── README.md (основная документация)
├── DEVELOPMENT.md (дорожная карта)
│
├── src/
│   ├── lib.rs (публичный API)
│   ├── backtest.rs (BacktestEngine)
│   ├── simulator.rs (MarketSimulator)
│   ├── models.rs (структуры данных)
│   └── tests.rs (unit tests)
│
└── examples/
    └── simple_backtest.rs (пример использования)
```

## Дополнительные документы

- **BACKTEST_INTEGRATION.md** - архитектура и workflow
- **backtest_engine/DEVELOPMENT.md** - дорожная карта разработки
- **backtest_engine/README.md** - техническая документация

## Как начать работать

1. Прочитайте `backtest_engine/README.md`
2. Изучите `examples/simple_backtest.rs`
3. Запустите тесты: `cargo test --package backtest_engine --verbose`
4. В файле `DEVELOPMENT.md` найдите, что нужно добавить дальше

## Заметки

- БД PostgreSQL: 79.174.88.198:16305
- Проверьте `.env` в `history_market_data/`
- Тесты работают без БД (используют mock-данные)
- Пример требует работающего подключения к БД

Удачи в разработке! 🚀
