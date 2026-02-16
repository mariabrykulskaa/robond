//! # History Market Data
//!
//! Модуль для чтения исторических данных и информации об облигациях из базы данных PostgreSQL.
//!
//! ## Архитектура
//!
//! Модуль разделён по ответственностям:
//!
//! | Слой | Компонент | Описание |
//! |---|---|---|
//! | Конфигурация | [`DbConfig`] | Подключение: env, файл, secret manager |
//! | Клиент | [`MarketDataClient`] | Запросы к БД, принимает `DbConfig` или готовый пул |
//! | Модели | [`models`] | Структуры, маппируемые на таблицы БД |
//! | Ошибки | [`Error`] | Типизированные ошибки через `thiserror` |
//!
//! ## Потокобезопасность
//!
//! `MarketDataClient: Send + Sync + Clone`.
//! Внутренний `PgPool` построен на `Arc`, поэтому клиент безопасно
//! разделять между потоками/задачами tokio. `Clone` — дешёвый (тот же пул).
//!
//! ## Подключение к БД
//!
//! 1. Скопируй [`.env.example`](../.env.example) в `.env`:
//!    ```bash
//!    cp .env.example .env
//!    ```
//! 2. Заполни `.env` реальными данными (`DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USERNAME`, `DB_PASSWORD`).
//! 3. Убедись, что `.env` есть в `.gitignore` (уже добавлен).
//!
//! ## Примеры
//!
//! ### Рекомендуемый способ (через `DbConfig`)
//!
//! ```no_run
//! use history_market_data::{DbConfig, MarketDataClient};
//! use chrono::NaiveDate;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), history_market_data::Error> {
//!     let config = DbConfig::from_env()?;
//!     let client = MarketDataClient::with_config(&config).await?;
//!
//!     let date = NaiveDate::from_ymd_opt(2025, 6, 11).unwrap();
//!     let candles = client.get_candles_by_date(date).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Быстрый способ (shortcut)
//!
//! ```no_run
//! use history_market_data::MarketDataClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), history_market_data::Error> {
//!     let client = MarketDataClient::from_env().await?;
//!     let bond = client.get_bond_by_isin("RU000A10BS76").await?;
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod models;

pub use client::MarketDataClient;
pub use config::DbConfig;
pub use error::{Error, Result};
pub use models::{BondCoupon, BondHistoryData, BondInfo, BondPayment};
