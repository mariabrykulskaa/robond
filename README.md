# Robond

Платформа для автоматической торговли облигациями на Московской бирже: бэктестирование стратегий на исторических данных, торговля в реальном времени через T-Invest API и веб-интерфейс для управления портфелем.

## Архитектура

```
┌─────────────┐    ┌──────────────────┐    ┌───────────────────┐
│   web       │───▶│   api_server     │───▶│   portfolio       │
│  (React)    │    │  (Axum REST API) │    │  (PostgreSQL)     │
└─────────────┘    └──────┬───────────┘    └───────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌──────────────┐ ┌────────────────┐ ┌──────────────────┐
│ live_engine   │ │ backtest_engine│ │history_market_data│
│ (T-Invest    │ │ (симуляция     │ │ (PostgreSQL,      │
│  gRPC API)   │ │  на истории)   │ │  данные MOEX)     │
└──────┬───────┘ └───────┬────────┘ └──────────────────┘
       │                 │
       ▼                 ▼
┌─────────────────────────────────┐
│       trading_strategies        │
│  (yield_maximizer, diversified, │
│   high_yield_short, ...)        │
└─────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────┐
│       t-invest-api-rust         │
│      (gRPC клиент к брокеру)    │
└─────────────────────────────────┘
```

## Модули

| Модуль | Описание |
|--------|----------|
| `history_market_data` | Клиент к PostgreSQL с историческими данными облигаций (свечи, купоны, выплаты) |
| `trading_strategies` | Торговые стратегии: `YieldMaximizer`, `DiversifiedShortDuration`, `HighYieldShort`, `MostProfitableBond` |
| `backtest_engine` | Движок бэктестирования: симуляция торговли, выплат, дефолтов и оферт на исторических данных |
| `live_engine` | Запуск стратегий в реальном времени через T-Invest API (sandbox / production) |
| `portfolio` | Учёт портфеля в PostgreSQL: позиции, кэш, снимки стоимости |
| `api_server` | REST API (Axum): авторизация, управление портфелями, запуск стратегий |
| `web` | Веб-интерфейс (React + TypeScript + Vite) |
| `t-invest-api-rust` | gRPC-клиент к T-Invest API (подмодуль) |

## Быстрый старт

### Требования

- Rust (stable)
- Node.js 18+ (для фронтенда)
- PostgreSQL (доступ к БД с историческими данными)
- Protobuf Compiler (`protoc`)

### Клонирование

Репозиторий содержит подмодули, поэтому при клонировании используйте флаг `--recursive`:

```bash
git clone --recursive git@github.com:robond-fintech/robond.git
cd robond
```

### Настройка окружения

Создайте файл `.env` в корне проекта:

```env
DB_HOST=...
DB_PORT=...
DB_NAME=...
DB_USERNAME=...
DB_PASSWORD=...

# Для live_engine и api_server
SANDBOX_TOKEN=t.ваш_sandbox_токен
ACCOUNT_ID=ваш_account_id
JWT_SECRET=ваш_секрет_для_jwt
```

### Сборка и тесты

```bash
cargo build --workspace
cargo test --workspace
```

### Запуск бэктеста

```bash
cargo run --package backtest_engine --example diversified_short_duration_backtest --release
```

### Запуск API сервера

```bash
cargo run --package api_server --release
```

### Запуск фронтенда

```bash
cd web
npm install
npm run dev
```

## Генерация документации

```bash
cargo doc --open --no-deps
```
