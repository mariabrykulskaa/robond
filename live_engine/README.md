# live_engine — Модуль торговли в реальном времени

Модуль для запуска торговых стратегий на реальном рынке (или в sandbox) через T-Invest API.

## Обзор

`live_engine` связывает торговую стратегию (`trading_strategies::Strategy`) с реальным брокерским API. Модуль:

1. Получает текущее состояние портфеля (свободные деньги, позиции по облигациям)
2. Загружает актуальные рыночные цены через T-Invest API
3. Подгружает справочную информацию по облигациям из БД (`history_market_data`)
4. Вызывает стратегию для принятия торговых решений
5. Выставляет рыночные ордера на покупку/продажу

## Архитектура

```
┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  T-Invest    │◄───►│   live_engine    │◄───►│ history_market  │
│  API (gRPC)  │     │                  │     │ _data (Postgres)│
└──────────────┘     │  get_portfolio() │     └─────────────────┘
                     │  get_prices()    │
                     │  make_orders()   │     ┌─────────────────┐
                     │  run()           │◄───►│ trading_        │
                     └──────────────────┘     │ strategies      │
                                              └─────────────────┘
```

## Зависимости

| Крейт                  | Назначение                                        |
|------------------------|---------------------------------------------------|
| `t-invest-api-rust`    | gRPC-клиент к брокерскому API (sandbox / prod)     |
| `trading_strategies`   | Трейт `Strategy` и типы (`Portfolio`, `MarketOrder`) |
| `history_market_data`  | Клиент к PostgreSQL с историческими данными         |
| `backtest_engine`      | Функция `build_bonds_info` для загрузки справочника |
| `chrono`               | Работа с датами                                    |
| `rust_decimal`         | Точная арифметика для финансовых вычислений         |
| `tokio`                | Асинхронный runtime                                |

## Быстрый старт

### 1. Переменные окружения

Создайте файл `.env` в корне проекта:

```env
# T-Invest API
SANDBOX_TOKEN=t.ваш_sandbox_токен
ACCOUNT_ID=ваш_account_id

# База данных (для загрузки справочника облигаций)
DB_HOST=...
DB_PORT=...
DB_NAME=...
DB_USERNAME=...
DB_PASSWORD=...
```

### 2. Минимальный пример

```rust
use chrono::NaiveDate;
use dotenvy::dotenv;
use live_engine::run;
use rust_decimal::Decimal;
use std::{collections::HashMap, env};
use t_invest_api_rust::{Client, EndPoint};
use trading_strategies::{BondPersistentInfo, Isin, MarketOrder, Portfolio, Strategy};

struct DoNothingStrategy;

impl Strategy for DoNothingStrategy {
    fn decide_trades(
        &self,
        _date: NaiveDate,
        _portfolio: &Portfolio,
        _bonds_info: &HashMap<Isin, BondPersistentInfo>,
        _bonds_prices: &HashMap<Isin, Decimal>,
        _bonds_volumes: &HashMap<Isin, i64>,
    ) -> Vec<MarketOrder> {
        vec![]
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let sandbox_token = env::var("SANDBOX_TOKEN").unwrap();
    let account_id = env::var("ACCOUNT_ID").unwrap();

    let mut client = Client::try_new(sandbox_token, EndPoint::Sandbox)
        .await
        .unwrap();

    run(&account_id, &mut client, DoNothingStrategy).await;
}
```

### 3. С реальной стратегией

```rust
use dotenvy::dotenv;
use live_engine::run;
use std::env;
use t_invest_api_rust::{Client, EndPoint};
use trading_strategies::yield_maximizer::YieldMaximizerStrategy;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("SANDBOX_TOKEN").unwrap();
    let account_id = env::var("ACCOUNT_ID").unwrap();

    let mut client = Client::try_new(token, EndPoint::Sandbox).await.unwrap();

    let strategy = YieldMaximizerStrategy::default();
    run(&account_id, &mut client, strategy).await;
}
```

> **⚠️ Важно:** Для реальной торговли замените `EndPoint::Sandbox` на `EndPoint::Prod` и используйте боевой токен.

## Публичный API

### `run<T: Strategy>(account_id, client, strategy)`

Главная точка входа. Выполняет один цикл принятия решений:

```rust
pub async fn run<T: Strategy>(
    account_id: &str,
    client: &mut Client,
    strategy: T,
)
```

**Параметры:**

| Параметр      | Тип             | Описание                                    |
|---------------|-----------------|---------------------------------------------|
| `account_id`  | `&str`          | Идентификатор брокерского счёта              |
| `client`      | `&mut Client`   | Подключённый gRPC-клиент T-Invest API        |
| `strategy`    | `T: Strategy`   | Реализация торговой стратегии                |

**Что делает:**

1. `get_portfolio()` — запрашивает позиции и баланс через `OperationsService/GetPositions`
2. `get_ticker_to_info()` — загружает справочник рублёвых облигаций через `InstrumentsService/Bonds`
3. `get_prices()` — получает последние биржевые цены через `MarketDataService/GetLastPrices`
4. `build_bonds_info()` — загружает детальную информацию (купоны, даты погашения) из PostgreSQL
5. `strategy.decide_trades()` — вызывает стратегию для формирования списка ордеров
6. `make_orders()` — отправляет рыночные ордера через `OrdersService/PostOrder`

## Внутренние функции

| Функция               | Описание                                                                                       |
|------------------------|-----------------------------------------------------------------------------------------------|
| `get_portfolio()`      | Получает свободные рубли и количество облигаций по тикерам                                     |
| `get_ticker_to_info()` | Загружает справочник всех рублёвых облигаций со статусом `Base`                                |
| `get_price()`          | Пересчитывает цену из пунктов в рубли: `points/100 × номинал + НКД`                          |
| `get_prices()`         | Получает последние биржевые цены для всех облигаций из справочника                             |
| `make_order()`         | Отправляет один рыночный ордер (Buy/Sell)                                                      |
| `make_orders()`        | Последовательно отправляет список ордеров                                                      |

## Формула расчёта цены

Цена облигации в рублях рассчитывается как:

$$
P = \frac{\text{points}}{100} \times \text{nominal} + \text{ACI}
$$

где:
- `points` — котировка в процентах от номинала
- `nominal` — текущий номинал облигации
- `ACI` — накопленный купонный доход

## Режимы работы

| Режим     | `EndPoint`          | Описание                                  |
|-----------|---------------------|-------------------------------------------|
| Sandbox   | `EndPoint::Sandbox` | Тестовая среда, сделки не исполняются      |
| Production| `EndPoint::Prod`    | Реальная торговля, ордера уходят на биржу  |

## Ограничения

- Модуль выполняет **один цикл** принятия решений (не запускает бесконечный loop)
- Объём для всех облигаций устанавливается как `1_000_000_000` (без реальных рыночных объёмов)
- Ордера выставляются строго **по рынку** (`OrderType::Market`)
- Поддерживаются только **рублёвые облигации**
- Нет обработки ошибок при выставлении ордеров (unwrap)
