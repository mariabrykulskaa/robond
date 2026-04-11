//! Модуль для работы торгового робота в реальном времени.
//!
//! Пример точки входа:
//!
//! ```no_run
//! use chrono::NaiveDate;
//! use live_engine::run;
//!
//! use dotenvy::dotenv;
//! use rust_decimal::Decimal;
//! use t_invest_api_rust::{Client, EndPoint};
//! use trading_strategies::{BondPersistentInfo, Isin, MarketOrder, Portfolio, Strategy};
//! use std::{collections::HashMap, env};
//!
//! /// Стратегия-заглушка: ничего не делает, позволяет проверить движок без логики.
//! struct DoNothingStrategy;
//!
//! impl Strategy for DoNothingStrategy {
//!     fn decide_trades(
//!         &self,
//!         _date: NaiveDate,
//!         _portfolio: &Portfolio,
//!         _bonds_info: &HashMap<Isin, BondPersistentInfo>,
//!         _bonds_prices: &HashMap<Isin, Decimal>,
//!         _bonds_volumes: &HashMap<Isin, i64>,
//!     ) -> Vec<MarketOrder> {
//!         vec![]
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     dotenv().ok();
//!     let sandbox_token = env::var("SANDBOX_TOKEN").unwrap();
//!     let account_id = env::var("ACCOUNT_ID").unwrap();
//!
//!     let mut client = Client::try_new(sandbox_token, EndPoint::Sandbox).await.unwrap();
//!
//!     run(&account_id, &mut client, DoNothingStrategy).await;
//! }
//! ```

use backtest_engine::build_bonds_info;
use rust_decimal::Decimal;
use std::collections::HashMap;
use t_invest_api_rust::{
    Client,
    decimal::{money_value_to_decimal, quotation_to_decimal},
    proto::{
        Bond, GetLastPricesRequest, InstrumentStatus, InstrumentsRequest, LastPriceType, OrderDirection, OrderType,
        PositionsRequest, PostOrderRequest, PriceType, Quotation,
    },
};
use trading_strategies::{Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

/// Получает состояние портфеля
pub async fn get_portfolio(client: &mut Client, account_id: &str) -> Result<Portfolio, String> {
    let response = client
        .operations
        .get_positions(PositionsRequest {
            account_id: account_id.to_string(),
        })
        .await
        .map_err(|e| format!("Failed to get positions: {e}"))?
        .into_inner();

    let rub = response
        .money
        .iter()
        .find(|m| m.currency == "rub")
        .map(money_value_to_decimal)
        .unwrap_or(Decimal::ZERO);

    let mut bonds_count = HashMap::<Isin, i64>::new();

    for security in response.securities.iter() {
        if security.instrument_type == "bond" {
            bonds_count.insert(security.ticker.clone(), security.balance);
        }
    }

    Ok(Portfolio {
        free_money: rub,
        bonds_count,
    })
}

pub async fn get_ticker_to_info(client: &mut Client) -> HashMap<String, Bond> {
    let mut request = InstrumentsRequest::default();
    request.set_instrument_status(InstrumentStatus::Base);
    let response = client.instruments.bonds(request).await.unwrap().into_inner();
    let mut ticker_to_info = HashMap::<String, Bond>::new();

    for bond_info in response.instruments {
        if bond_info.currency != "rub" {
            continue;
        }
        let opt = ticker_to_info.insert(bond_info.ticker.clone(), bond_info);
        assert_eq!(opt, None);
    }

    ticker_to_info
}

pub fn get_price(points: Quotation, bond_info: &Bond) -> Option<Decimal> {
    let points = quotation_to_decimal(points);
    if !bond_info.buy_available_flag || !bond_info.sell_available_flag {
        return None;
    }

    let nominal = bond_info.nominal.clone().unwrap();
    let aci_value = bond_info.aci_value.clone().unwrap();
    let price = points / Decimal::from(100) * money_value_to_decimal(&nominal) + money_value_to_decimal(&aci_value);

    Some(price)
}

pub async fn get_prices(client: &mut Client, ticker_to_info: &HashMap<String, Bond>) -> HashMap<String, Decimal> {
    // Build figi → ticker mapping and use figi as instrument_id
    let mut figi_to_ticker = HashMap::<String, String>::new();
    let mut ids = Vec::<String>::new();
    for (ticker, bond_info) in ticker_to_info {
        figi_to_ticker.insert(bond_info.figi.clone(), ticker.clone());
        ids.push(bond_info.figi.clone());
    }
    ids.sort();

    let mut request = GetLastPricesRequest {
        instrument_id: ids,
        ..GetLastPricesRequest::default()
    };
    request.set_last_price_type(LastPriceType::LastPriceExchange);
    request.set_instrument_status(InstrumentStatus::Base);
    let response = client.market_data.get_last_prices(request).await.unwrap().into_inner();
    let last_prices = response.last_prices;

    let mut prices = HashMap::<String, Decimal>::new();
    for last_price in last_prices {
        // Resolve ticker from figi
        let ticker = if !last_price.ticker.is_empty() {
            last_price.ticker.clone()
        } else if let Some(t) = figi_to_ticker.get(&last_price.figi) {
            t.clone()
        } else {
            continue;
        };

        if let Some(bond_info) = ticker_to_info.get(&ticker) {
            if let Some(points) = last_price.price {
                if let Some(price) = get_price(points, bond_info) {
                    prices.insert(ticker, price);
                }
            }
        }
    }

    prices
}

pub async fn make_order(
    client: &mut Client,
    order: &MarketOrder,
    ticker_to_info: &HashMap<String, Bond>,
    account_id: &str,
) {
    let bond_info = ticker_to_info.get(&order.isin).unwrap();
    let mut request = PostOrderRequest {
        quantity: order.count,
        account_id: account_id.to_string(),
        instrument_id: bond_info.figi.clone(),
        ..PostOrderRequest::default()
    };
    request.set_direction(match order.order_type {
        MarketOrderType::Buy => OrderDirection::Buy,
        MarketOrderType::Sell => OrderDirection::Sell,
    });
    request.set_order_type(OrderType::Market);
    request.set_price_type(PriceType::Currency);

    client.orders.post_order(request).await.unwrap().into_inner();
}

pub async fn make_orders(
    client: &mut Client,
    orders: &Vec<MarketOrder>,
    ticker_to_info: &HashMap<String, Bond>,
    account_id: &str,
) {
    for order in orders {
        make_order(client, order, ticker_to_info, account_id).await;
    }
}

use chrono::prelude::*;

use history_market_data::MarketDataClient;

pub async fn run<T: Strategy>(account_id: &str, client: &mut Client, stgategy: T) {
    let portfolio = get_portfolio(client, account_id).await.expect("failed to get portfolio");
    let ticker_to_info = get_ticker_to_info(client).await;
    let prices = get_prices(client, &ticker_to_info).await;

    let md_client = MarketDataClient::from_env().await.unwrap();

    let bonds_info = build_bonds_info(&md_client).await.unwrap();

    let mut bonds_volumes = HashMap::<String, i64>::new();
    for (ticker, _bond_info) in bonds_info.iter() {
        bonds_volumes.insert(ticker.to_string(), 1_000_000_000);
    }

    let orders = stgategy.decide_trades(
        Local::now().date_naive(),
        &portfolio,
        &bonds_info,
        &prices,
        &bonds_volumes,
    );
    make_orders(client, &orders, &ticker_to_info, account_id).await;
}
