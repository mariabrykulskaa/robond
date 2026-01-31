//! # History Market Data
//!
//! Модуль для чтения исторических данных и информации об облигациях из базы данных PostgreSQL.
//!
//! ## Основной функционал
//!
//! - Получение исторических свечей по облигациям за определенную дату или период
//! - Получение информации об облигациях из базы данных
//! - Поиск облигаций по ISIN коду
//! - Поддержка различных способов подключения к базе данных
//!
//! ## Примеры использования
//!
//! ### Интерактивное подключение
//!
//! ```no_run
//! use history_market_data::MarketDataClient;
//! use chrono::NaiveDate;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Подключение с интерактивным вводом логина и пароля
//!     let client = MarketDataClient::connect_interactive(
//!         "79.174.88.198",
//!         16305,
//!         "HedgehogFinanceDB"
//!     ).await?;
//!     
//!     // Получение свечей за дату
//!     let date = NaiveDate::from_ymd_opt(2025, 6, 11).unwrap();
//!     let candles = client.get_candles_by_date(date).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Подключение с учетными данными
//!
//! ```no_run
//! use history_market_data::MarketDataClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = MarketDataClient::from_credentials(
//!         "79.174.88.198",
//!         16305,
//!         "HedgehogFinanceDB",
//!         "username",
//!         "password"
//!     ).await?;
//!     
//!     // Поиск облигации по ISIN
//!     let bond = client.get_bond_by_isin("RU000A10BS76").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod models;

pub use client::MarketDataClient;
pub use models::{BondHistoryData, BondInfo};
