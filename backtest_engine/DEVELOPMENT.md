# Разработка backtest_engine

## Дорожная карта

### Phase 1: Core (✓ Завершено)
- [x] Структуры данных (TradeEvent, PaymentEvent, BacktestResult)
- [x] MarketSimulator (базовое исполнение ордеров, обработка платежей)
- [x] BacktestEngine (координация, загрузка данных)
- [x] Unit tests

### Phase 2: Payment Integration (На разработку)
Необходимо добавить запросы к БД для получения платежей:

```sql
-- Пример запроса платежей по облигации
SELECT date, amount, payment_type
FROM bond_payment
WHERE bond_id = ?
AND date BETWEEN ? AND ?
ORDER BY date;
```

**Задачи:**
- [ ] Добавить методы в `MarketDataClient`:
  - `get_bond_payments(bond_id, start_date, end_date)`
  - `get_coupon_schedule(coupon_id)`
- [ ] Интегрировать в `BacktestEngine::run_backtest()`
- [ ] Обработать события платежей по датам

### Phase 3: Strategy Integration (На разработку)
Подключение торговых стратегий из модуля `trading_strategies`:

```rust
pub trait TradingStrategy {
    fn generate_orders(
        &self,
        bond_info: &BondInfo,
        recent_candles: &[BondHistoryData],
        portfolio: &Portfolio,
    ) -> Vec<MarketOrder>;
}
```

**Задачи:**
- [ ] Определить трейт `TradingStrategy`
- [ ] Добавить параметр стратегии в `BacktestEngine::new()`
- [ ] Вызывать `strategy.generate_orders()` в цикле симуляции
- [ ] Примеры стратегий (простая, momentum, value)

### Phase 4: Advanced Metrics (На разработку)
Расширенный анализ результатов:

- [ ] Sharpe Ratio
- [ ] Sortino Ratio
- [ ] Maximum Drawdown
- [ ] Calmar Ratio
- [ ] Win Rate
- [ ] Profit Factor

### Phase 5: Corporate Actions (На разработку)
- [ ] Обработка дефолтов
- [ ] Переиндексирование номиналов
- [ ] Срочные события

## Текущая архитектура

```
BacktestEngine
  │
  ├─→ MarketDataClient (БД)
  │    ├─ get_candles_by_date()
  │    ├─ get_bond_info()
  │    ├─ get_bond_by_isin()
  │    └─ get_all_bonds()
  │
  └─→ MarketSimulator
       ├─ cache_prices()
       ├─ execute_order()
       ├─ process_payment()
       └─ get_portfolio_value()
```

Запрос на фазу 2:
- `MarketDataClient.get_bond_payments(bond_id)`

Запрос на фазу 3:
- Интерфейс `TradingStrategy`

## Как запустить

### Сборка
```bash
cd /root/robond
cargo build --package backtest_engine
```

### Тесты
```bash
cargo test --package backtest_engine
```

### Пример
```bash
cargo run --package backtest_engine --example simple_backtest
```

## Структура файлов

```
backtest_engine/
├── Cargo.toml           # Зависимости
├── README.md            # Основная документация
├── src/
│   ├── lib.rs           # Публичный API
│   ├── backtest.rs      # BacktestEngine
│   ├── simulator.rs     # MarketSimulator
│   ├── models.rs        # Структуры данных
│   └── tests.rs         # Unit tests
└── examples/
    └── simple_backtest.rs  # Пример использования
```

## Тестовые данные

Для локального тестирования используйте:
- `.env.example` → `.env` в `history_market_data/`
- PostgreSQL сервер: 79.174.88.198:16305
- БД: HedgehogFinanceDB

## Известные ограничения

1. **Нет комиссий**: Не учитываются брокерские комиссии и спреды
2. **Бесконечная ликвидность**: Предполагается, что можно купить/продать любой объём
3. **Mid-price**: Используется средняя цена свечи, нет slip-модели
4. **Нет маржи**: Нет кредитного плеча
5. **Нет синхронизации по времени**: Все сделки дневные

## Контакты

- Модуль интеграции: `history_market_data/src/client.rs`
- Примеры стратегий: `trading_strategies/`
