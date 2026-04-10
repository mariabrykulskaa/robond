use dotenvy::dotenv;
use std::env;
use t_invest_api_rust::{
    EndPoint,
    proto::{InstrumentStatus, InstrumentsRequest},
};

use bonds_db_client;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let sandbox_tokens = env::var("SANDBOX_TOKENS").unwrap();
    let sandbox_tokens: Vec<String> = serde_json::from_str(&sandbox_tokens).unwrap();
    let sandbox_token = sandbox_tokens[0].clone();

    let mut t_bank_client = t_invest_api_rust::Client::try_new(sandbox_token, EndPoint::Sandbox)
        .await
        .unwrap();

    let mut request = InstrumentsRequest::default();
    request.set_instrument_status(InstrumentStatus::All);
    let response = t_bank_client.instruments.bonds(request).await.unwrap().into_inner();
    let bonds = response.instruments;

    dbg!(bonds.len());

    let database_url = env::var("DATABASE_URL").unwrap();
    let db_client = bonds_db_client::Client::new(&database_url).await.unwrap();
    db_client.bonds.clear().await.unwrap();
    db_client.bonds.insert(&bonds).await.unwrap();
}
