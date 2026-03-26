# Market Data Module

Модуль для чтения исторических данных и информации об облигациях из базы данных PostgreSQL.

## Функционал

- Получение всех свечей (исторических данных), соответствующих заданному дню
- Получение исторических данных для конкретной облигации
- Получение неизменяемой информации об облигациях
- Получение информации о купоне облигации
- Получение фактических выплат по облигации за диапазон дат
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

### BondCoupon
Информация о купоне из таблицы `bond_coupon`:
- `id` - ID записи купона
- `description` - текстовое описание
- `size` - размер купона
- `aci` - накопленный купонный доход
- `period` - период купона в днях
- `type_id` - тип купона
- `sum` - сумма выплаты

### BondPayment
Фактическая выплата из таблицы `bond_payment`:
- `id` - ID записи выплаты
- `date` - дата выплаты
- `size` - сумма выплаты
- `relative_size` - относительный размер в процентах
- `bond_id` - ID облигации
- `currency_id` - ID валюты
- `type_id` - тип выплаты

## API

### Подключение к базе данных

**`MarketDataClient::from_env()`** - единственный способ подключения

Читает учетные данные из `.env` файла:
- `DB_HOST` - Хост базы данных
- `DB_PORT` - Порт подключения
- `DB_NAME` - Имя базы данных
- `DB_USERNAME` - Имя пользователя
- `DB_PASSWORD` - Пароль

**Инструкция:**

1. Скопируй `.env.example` в `.env`:
   ```bash
   cp .env.example .env
   ```

2. Отредактируй `.env` с реальными данными подключения:
   ```env
   DB_HOST=79.174.88.198
   DB_PORT=16305
   DB_NAME=HedgehogFinanceDB
   DB_USERNAME=your_username
   DB_PASSWORD=your_password
   ```

3. Используй в коде:
   ```rust
   let client = MarketDataClient::from_env().await?;
   ```

Учетные данные хранятся в `.env`, который находится в `.gitignore` для безопасности.

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

### Получение купонов и выплат

**`get_coupon_info(coupon_id: i64)`**
- Параметры: ID купона
- Возвращает: `Result<Option<BondCoupon>>` - информация о записи в `bond_coupon`

**`get_bond_payments(bond_id: i64, start_date: NaiveDate, end_date: NaiveDate)`**
- Параметры: ID облигации, начальная дата, конечная дата
- Возвращает: `Result<Vec<BondPayment>>` - выплаты по облигации за период, отсортированные по дате

### Получение информации об облигациях

**`get_bond_info(bond_id: i64)`**
- Параметры: ID облигации
- Возвращает: `Result<Option<BondInfo>>` - информация об облигации

**`get_bond_by_isin(isin: &str)`**
- Параметры: ISIN код
- Возвращает: `Result<Option<BondInfo>>` - поиск облигации по ISIN

**`get_all_bonds(limit: Option<i64>, offset: Option<i64>)`**
- Параметры: лимит (если `None`, вернуть все записи), смещение (по умолчанию 0)
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
- `dotenvy` - загрузка переменных окружения из .env файла
