//! # Portfolio
//!
//! Модуль учёта портфеля облигаций с хранением в PostgreSQL.
//!
//! ## Функциональность
//!
//! - Хранение количества облигаций каждого типа (ISIN)
//! - Хранение суммы свободных денежных средств
//! - Вычисление рыночной стоимости портфеля по текущим ценам
//! - Запись и чтение снимков стоимости портфеля (для графика и расчёта доходности)
//!
//! ## Быстрый старт
//!
//! ```no_run
//! use portfolio::PortfolioClient;
//!
//! # async fn example() -> portfolio::Result<()> {
//! let client = PortfolioClient::from_env().await?;
//! client.run_migrations().await?;
//!
//! let p = client.create_portfolio("Мой портфель").await?;
//! client.set_cash(p.id, rust_decimal_macros::dec!(1_000_000), "RUB").await?;
//! client.set_holding(p.id, "RU000A0ZZBC7", 100).await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod models;

pub use client::PortfolioClient;
pub use error::{Error, Result};
pub use models::{Portfolio, PortfolioCash, PortfolioHolding, PortfolioSnapshot};
