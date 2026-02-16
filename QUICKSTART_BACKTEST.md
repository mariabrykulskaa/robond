# Quickstart: Backtest Engine

## За 5 минут

### 1. Проверка структуры
```bash
ls -la /root/robond/backtest_engine/
# Должны быть: Cargo.toml, README.md, src/, examples/
```

### 2. Запуск тестов (без БД)
```bash
cd /root/robond
cargo test --package backtest_engine 2>&1 | grep -E "^test|passed|PASSED"
```

### 3. Пример использования
```bash
cd /root/robond

# Если БД доступна:
cargo run --package backtest_engine --example simple_backtest

# Вывод будет примерно такой:
# === Backtest Engine Example ===
# [1/3] Подключаюсь к БД...
# [2/3] Инициализирую движок бэктеста...
# [3/3] Запускаю симуляцию историческихданных...
# === РЕЗУЛЬТАТЫ БЭКТЕСТА ===
# Начальный капитал:      1000000.00 руб
# Финальная стоимость:    1000000.00 руб
# Прибыль/Убыток:             0.00 руб  
# Возврат:                    0.00 %
```

### 4. Посмотреть результаты
```bash
cat backtest_result.json | jq '.' | head -30
```

## Основной API

### Создание и запуск
```rust
use backtest_engine::BacktestEngine;
use history_market_data::MarketDataClient;
use chrono::NaiveDate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MarketDataClient::from_env().await?;
    let engine = BacktestEngine::new(
        client,
        1_000_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
    );
    let result = engine.run_backtest().await?;
    println!("Возврат: {:.2}%", result.return_percent);
    Ok(())
}
```

### Прямая работа с симулятором
```rust
use backtest_engine::MarketSimulator;
use trading_strategies::{MarketOrder, MarketOrderType};

let mut sim = MarketSimulator::new(1_000_000, date);

// Кешируем цены
sim.cache_prices("RU000A104H08".to_string(), 98.0, 99.0, 97.0, 100.0, 5000.0, 1000.0);

// Покупаем 10 облигаций
let order = MarketOrder {
    isin: "RU000A104H08".to_string(),
    order_type: MarketOrderType::Buy,
    count: 10,
};
let trade = sim.execute_order(order, true)?;
println!("Куплено: {} шт по {:.2}%", trade.quantity, trade.price);

// Обрабатываем выплату купона
sim.process_payment("RU000A104H08".to_string(), 5.0, "coupon".to_string());
```

## Структура файлов

### Для начинающих
1. Прочитайте: `BACKTEST_README.md`
2. Изучите: `backtest_engine/examples/simple_backtest.rs`
3. Запустите: `cargo test --package backtest_engine`

### Для разработчиков
1. Архитектура: `BACKTEST_INTEGRATION.md`
2. Детали: `backtest_engine/README.md`
3. Дорожная карта: `backtest_engine/DEVELOPMENT.md`

### Для интеграции
- `backtest_engine/src/lib.rs` - публичный API
- `backtest_engine/src/simulator.rs` - MarketSimulator
- `backtest_engine/src/backtest.rs` - BacktestEngine

## Что работает

✅ Покупка/продажа облигаций ('buy'/'sell')
✅ Обработка платежей (купоны, погашения)
✅ Расчёт портфеля и метрик
✅ Полная история сделок
✅ Экспорт результатов в JSON

## Что нужно добавить

❌ Платежи из БД (Phase 2)
❌ Стратегии (Phase 3)
❌ Продвинутые метрики (Phase 4)
❌ Корпоративные события (Phase 5)

## Типичные вопросы

**Q: Как запустить без БД?**
A: Юнит-тесты работают без БД: `cargo test --package backtest_engine`

**Q: Как использовать со своей стратегией?**
A: Планируется Phase 3. Пока что используйте `MarketSimulator::execute_order()` напрямую.

**Q: Как получить платежи?**
A: Phase 2 - запросы к БД. Сейчас обрабатываются вручную через `process_payment()`.

**Q: Как сохранить результаты?**
A: JSON экспортируется автоматически в `backtest_result.json`

## Частые ошибки

```
error: cannot pull with rebase: You have unstaged changes
→ git add .gitignore && git commit -m "fix"

error: could not compile `backtest_engine`
→ cargo clean && cargo build --package backtest_engine

error: Connection refused
→ Проверьте .env в history_market_data/
```

## Полезные команды

```bash
# Компиляция
cargo build --package backtest_engine

# Тесты
cargo test --package backtest_engine
cargo test --package backtest_engine -- --nocapture  # с выводом

# Пример
cargo run --package backtest_engine --example simple_backtest

# Проверка примечаний
cargo clippy --package backtest_engine

# Документация
cargo doc --package backtest_engine --open
```

---

**Готово к использованию!** Начните с `cargo test` 🚀
