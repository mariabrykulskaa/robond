use dotenvy::dotenv;
use std::{env, time::Instant};
use t_invest_api_rust::{
    EndPoint,
    proto::{GetBondEventsRequest, GetBondEventsResponse},
};
use timestamp_utils::{MAX_TIMESTAMP, MIN_TIMESTAMP};
use uuid::Uuid;

mod requests;

use bonds_db_client;

use async_trait::async_trait;

use crate::requests::{Request, send_requests};

#[async_trait]
impl Request for GetBondEventsRequest {
    type Response = GetBondEventsResponse;

    async fn send(
        &self,
        client: &mut t_invest_api_rust::Client,
    ) -> Result<tonic::Response<Self::Response>, tonic::Status> {
        client.instruments.get_bond_events(self.clone()).await
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let sandbox_tokens = env::var("SANDBOX_TOKENS").unwrap();
    let sandbox_tokens: Vec<String> = serde_json::from_str(&sandbox_tokens).unwrap();
    let mut t_bank_clients = Vec::new();
    for sandbox_token in sandbox_tokens {
        let t_bank_client = t_invest_api_rust::Client::try_new(sandbox_token, EndPoint::Sandbox)
            .await
            .unwrap();
        t_bank_clients.push(t_bank_client);
    }

    let database_url = env::var("DATABASE_URL").unwrap();
    let db_client = bonds_db_client::Client::new(&database_url).await.unwrap();

    let bonds = db_client.bonds.read().await.unwrap();

    let requests = bonds
        .iter()
        .map(|bond| GetBondEventsRequest {
            from: Some(MIN_TIMESTAMP),
            to: Some(MAX_TIMESTAMP),
            instrument_id: bond.uid.to_string(),
            ..GetBondEventsRequest::default()
        })
        .collect::<Vec<GetBondEventsRequest>>();

    //let mut requests = requests;

    let start = Instant::now();
    let responses = send_requests(&requests, &mut t_bank_clients, 10).await;
    let duration = start.elapsed();

    println!("Время выполнения: {:?}", duration);
    let rps = requests.len() as f64 / duration.as_secs_f64();
    println!("Запросов в секунду: {}", rps);

    let instrument_uids = requests
        .iter()
        .map(|request| Uuid::parse_str(&request.instrument_id).unwrap())
        .collect::<Vec<Uuid>>();
    let events = responses
        .into_iter()
        .map(|response| response.events)
        .collect::<Vec<_>>();

    db_client.events.clear().await.unwrap();
    db_client.events.insert(&instrument_uids, &events).await.unwrap();
}
