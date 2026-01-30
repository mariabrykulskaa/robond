# Market Data Module

Модуль для чтения исторических данных и информации об облигациях из базы данных PostgreSQL.

## Функционал

- Получение всех свечей (исторических данных), соответствующих заданному дню
- Получение исторических данных для конкретной облигации
- Получение неизменяемой информации об облигациях
- Поиск облигаций по ISIN коду
- Получение списка всех облигаций с пагинацией

## Структуры данных

### BondHistoryData
Исторические данные по облигации (свечи) из таблицы `bond_bondhistorydata`:
- `id` - ID записи
- `date` - Дата торгов
- `num_trades` - Количество сделок
- `value` - Объем торгов в валюте
- `low` - Минимальная цена (% от номинала)
- `high` - Максимальная цена (% от номинала)
- `close` - Цена закрытия (% от номинала)
- `open` - Цена открытия (% от номинала)
- `volume` - Объем торгов в штуках
- `facevalue` - Номинальная стоимость
- `accint` - Накопленный купонный доход
- `full_information` - Полная информация из MOEX (JSON)
- `bond_id` - ID облигации

### BondInfo
Информация об облигации из таблицы `bond_bond`:
- `id` - ID облигации
- `isin` - ISIN код
- `title` - Название
- `is_subordinated` - Субординированная
- `placement_date` - Дата размещения
- `maturity_date` - Дата погашения
- `current_yield` - Текущая доходность
- `yield_to_maturity` - Доходность к погашению
- И другие поля...

## API

### Подключение к базе данных

**`MarketDataClient::new(database_url: &str)`**
- Параметры: URL подключения в формате `postgresql://username:password@host:port/database`
- Возвращает: `Result<MarketDataClient>`

**`MarketDataClient::from_credentials(host: &str, port: u16, database: &str, username: &str, password: &str)`**
- Параметры: хост, порт, имя БД, логин, пароль
- Возвращает: `Result<MarketDataClient>`

**`MarketDataClient::connect_interactive(host: &str, port: u16, database: &str)`**
- Параметры: хост, порт, имя БД
- Запрашивает логин и пароль из консоли (рекомендуется для публичных репозиториев)
- Возвращает: `Result<MarketDataClient>`

### Получение исторических данных

**`get_candles_by_date(date: NaiveDate)`**
- Параметры: дата
- Возвращает: `Result<Vec<BondHistoryData>>` - все свечи за указанную дату

**`get_bond_candle(bond_id: i64, date: NaiveDate)`**
- Параметры: ID облигации, дата
- Возвращает: `Result<Option<BondHistoryData>>` - свеча для конкретной облигации за дату

**`get_bond_candles_range(bond_id: i64, start_date: NaiveDate, end_date: NaiveDate)`**
- Параметры: ID облигации, начальная дата, конечная дата
- Возвращает: `Result<Vec<BondHistoryData>>` - история за период (отсортировано по дате)

### Получение информации об облигациях

**`get_bond_info(bond_id: i64)`**
- Параметры: ID облигации
- Возвращает: `Result<Option<BondInfo>>` - информация об облигации

**`get_bond_by_isin(isin: &str)`**
- Параметры: ISIN код
- Возвращает: `Result<Option<BondInfo>>` - поиск облигации по ISIN

**`get_all_bonds(limit: Option<i64>, offset: Option<i64>)`**
- Параметры: лимит (по умолчанию 1000), смещение (по умолчанию 0)
- Возвращает: `Result<Vec<BondInfo>>` - список облигаций с пагинацией

**`get_traded_bonds()`**
- Параметры: нет
- Возвращает: `Result<Vec<BondInfo>>` - только торгуемые облигации (`is_traded = true`)


## Зависимости

- `sqlx` - работа с PostgreSQL
- `tokio` - асинхронный runtime
- `chrono` - работа с датами
- `serde` - сериализация/десериализация
- `anyhow` - обработка ошибок
