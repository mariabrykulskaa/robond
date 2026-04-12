use dotenvy::dotenv;
use std::{env, time::Instant};
use t_invest_api_rust::{
    EndPoint,
    proto::{Coupon, GetBondCouponsRequest, GetBondCouponsResponse, GetBondEventsRequest},
};
use timestamp_utils::{MAX_TIMESTAMP, MIN_TIMESTAMP, timestamp_to_datetime};
use uuid::Uuid;

mod requests;

use bonds_db_client;

const TICKER: &str = "RU000A1062L7";

#[tokio::main]
async fn main() {
    dotenv().ok();
    let sandbox_tokens = env::var("SANDBOX_TOKENS").unwrap();
    let sandbox_tokens: Vec<String> = serde_json::from_str(&sandbox_tokens).unwrap();
    let sandbox_token = sandbox_tokens[0].clone();

    let mut t_bank_client = t_invest_api_rust::Client::try_new(sandbox_token, EndPoint::Sandbox)
        .await
        .unwrap();
    let database_url = env::var("DATABASE_URL").unwrap();
    let db_client = bonds_db_client::Client::new(&database_url).await.unwrap();

    let bonds = db_client.bonds.read().await.unwrap();
    let bonds = bonds.iter().filter(|&bond| bond.ticker == TICKER).collect::<Vec<_>>();
    assert_eq!(bonds.len(), 1);
    let instrument_uid = bonds[0].uid.clone();

    let request = GetBondEventsRequest {
        from: Some(MIN_TIMESTAMP),
        to: Some(MAX_TIMESTAMP),
        instrument_id: instrument_uid,
        ..GetBondEventsRequest::default()
    };
    let response: t_invest_api_rust::proto::GetBondEventsResponse = t_bank_client
        .instruments
        .get_bond_events(request)
        .await
        .unwrap()
        .into_inner();
    let events = response.events;
    for bond_event in events.iter() {
        let datetime = timestamp_to_datetime(bond_event.event_date.unwrap());
        println!("{}", datetime);
        println!("{:?}", bond_event.event_type());
        println!();
    }
}
