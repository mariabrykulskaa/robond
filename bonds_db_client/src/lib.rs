//! Клиент базы данных bonds_db
//! # Пример
//!
//! ```no_run
//! use bonds_db_client::{Client, ClientConfig};
//! use t_invest_api_rust::decimal::money_value_to_decimal;
//! use timestamp_utils::timestamp_to_datetime;
//!
//! const TICKER: &str = "RU000A1062L7";
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::new(&ClientConfig::from_env()).await.unwrap();
//!     let bonds = client.bonds.read().await.unwrap();
//!     let bonds = bonds.iter().filter(|&bond| bond.ticker == TICKER).collect::<Vec<_>>();
//!     let [bond] = bonds.try_into().unwrap();
//!     let instrument_uid = bond.uid.clone();
//!
//!     let coupons = client.coupons.read().await.unwrap();
//!     let coupons = coupons.iter().filter(|(uid, _coupons)| uid.to_string() == instrument_uid).collect::<Vec<_>>();
//!     let [(_, coupons)] = coupons.try_into().unwrap();
//!
//!     for coupon in coupons {
//!         let timestamp = coupon.coupon_date.unwrap();
//!         let datetime = timestamp_to_datetime(timestamp);
//!         println!("{}", datetime);
//!         let money_value = coupon.pay_one_bond.as_ref().unwrap();
//!         let pay_one_bond = money_value_to_decimal(money_value);
//!         println!("{}", pay_one_bond);
//!     }
//!     println!();
//!
//!     let events = client.events.read().await.unwrap();
//!     let events = events.iter().filter(|(uid, _events)| uid.to_string() == instrument_uid).collect::<Vec<_>>();
//!     let [(_, events)] = events.try_into().unwrap();
//!     for event in events {
//!         let datetime = timestamp_to_datetime(event.event_date.unwrap());
//!         println!("{}", datetime);
//!         println!("{:?}", event.event_type());
//!     }
//! }
//! ```

pub mod bonds_table_client;
pub mod coupons_table_client;
mod error;
pub mod events_table_client;
pub mod user_manager;

use std::env;

use crate::{
    bonds_table_client::BondsTableClient, coupons_table_client::CouponsTableClient,
    events_table_client::EventsTableClient, user_manager::UserManager,
};

use sqlx::postgres::PgPoolOptions;

pub use error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Максимальное число соединений в пуле (по умолчанию 5).
    pub max_connections: u32,
    pub ssl_root_cert_path: String,
}

impl ClientConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            host: env::var("BONDS_DB_HOST").unwrap(),
            port: env::var("BONDS_DB_PORT").unwrap().parse::<u16>().unwrap(),
            username: env::var("BONDS_DB_USERNAME").unwrap(),
            password: env::var("BONDS_DB_PASSWORD").unwrap(),
            max_connections: 5,
            ssl_root_cert_path: env::var("BONDS_DB_SSL_ROOT_CERT_PATH").unwrap(),
        }
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/bonds_db?sslmode=verify-full&sslrootcert={}",
            self.username, self.password, self.host, self.port, self.ssl_root_cert_path
        )
    }
}

pub struct Client {
    pub bonds: BondsTableClient,
    pub coupons: CouponsTableClient,
    pub events: EventsTableClient,
    pub user: UserManager,
}

impl Client {
    pub async fn new(config: &ClientConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.database_url())
            .await?;
        Ok(Client {
            bonds: BondsTableClient::new(pool.clone()),
            coupons: CouponsTableClient::new(pool.clone()),
            events: EventsTableClient::new(pool.clone()),
            user: UserManager::new(pool),
        })
    }
}
