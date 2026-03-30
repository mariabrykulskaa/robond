//! Модуль для бэктестирования торговых стратегий на исторических данных облигаций
//!
//! Позволяет:
//! - Симулировать торговлю облигациями на историческим данных
//! - Рассчитывать выплаты по купонам и номиналу
//! - Оценивать портфель и прибыль/убыток

pub mod backtest;
pub mod models;
pub mod simulator;

#[cfg(test)]
mod tests;

pub use backtest::BacktestEngine;
pub use backtest::build_bonds_info;
pub use models::{BacktestResult, TradeSimulation};
pub use simulator::MarketSimulator;
