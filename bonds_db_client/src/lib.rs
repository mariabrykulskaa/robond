//! Клиент базы данных bonds_db

pub mod bonds_table_client;
pub mod coupons_table_client;
mod error;
pub mod events_table_client;

use crate::{
    bonds_table_client::BondsTableClient, coupons_table_client::CouponsTableClient,
    events_table_client::EventsTableClient,
};

use sqlx::postgres::PgPoolOptions;

pub use error::{Error, Result};

pub struct Client {
    pub bonds: BondsTableClient,
    pub coupons: CouponsTableClient,
    pub events: EventsTableClient,
}

impl Client {
    pub async fn new(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;
        Ok(Client {
            bonds: BondsTableClient::new(pool.clone()),
            coupons: CouponsTableClient::new(pool.clone()),
            events: EventsTableClient::new(pool),
        })
    }
}
