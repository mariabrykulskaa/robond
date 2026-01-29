//! Этот крейт предоставляет удобный интерфейс для взаимодействия с T-Invest API по протоколу gRPC.
//!
//! # TODO
//!
//! - Улучшить обработку ошибок (заменить unwrap на Result)
//!
//! # Пример
//!
//! ```no_run
//! use std::fs;
//! use t_invest_api_rust::{
//!     Client, EndPoint,
//!     proto::{GetInfoRequest, GetLastPricesRequest, InstrumentStatus, LastPriceType},
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let auth_token = fs::read_to_string("sandbox_token.txt").unwrap();
//!     let mut client = Client::new(auth_token, EndPoint::Sandbox).await;
//!
//!     let request = GetInfoRequest {};
//!     let request = client.new_request(request);
//!     let response = client.users.get_info(request).await.unwrap().into_inner();
//!     dbg!(&response);
//!
//!     let mut request = GetLastPricesRequest {
//!         instrument_id: vec!["T_TQBR".to_string()],
//!         ..GetLastPricesRequest::default()
//!     };
//!     request.set_last_price_type(LastPriceType::LastPriceExchange);
//!     request.set_instrument_status(InstrumentStatus::Base);
//!     let request = client.new_request(request);
//!     let response = client.market_data.get_last_prices(request).await.unwrap().into_inner();
//!     dbg!(&response);
//! }
//! ```

#![allow(clippy::all)]
/// Код, сгенерированный из protobuf-контракта T-Invest API
pub mod proto {
    tonic::include_proto!("tinkoff.public.invest.api.contract.v1");
}

use tonic::{
    Request,
    transport::{Channel, ClientTlsConfig, Endpoint},
};

// Подключаем клиенты для всех сервисов
use proto::{
    instruments_service_client::InstrumentsServiceClient, market_data_service_client::MarketDataServiceClient,
    market_data_stream_service_client::MarketDataStreamServiceClient,
    operations_service_client::OperationsServiceClient,
    operations_stream_service_client::OperationsStreamServiceClient, orders_service_client::OrdersServiceClient,
    orders_stream_service_client::OrdersStreamServiceClient, sandbox_service_client::SandboxServiceClient,
    signal_service_client::SignalServiceClient, stop_orders_service_client::StopOrdersServiceClient,
    users_service_client::UsersServiceClient,
};

/// Клиент для взаимодействия с T-Invest API
#[derive(Clone)]
pub struct Client {
    auth_token: String,
    pub instruments: InstrumentsServiceClient<Channel>,
    pub market_data: MarketDataServiceClient<Channel>,
    pub market_data_stream: MarketDataStreamServiceClient<Channel>,
    pub operations: OperationsServiceClient<Channel>,
    pub operations_stream: OperationsStreamServiceClient<Channel>,
    pub orders: OrdersServiceClient<Channel>,
    pub orders_stream: OrdersStreamServiceClient<Channel>,
    pub sandbox: SandboxServiceClient<Channel>,
    pub signal: SignalServiceClient<Channel>,
    pub stop_orders: StopOrdersServiceClient<Channel>,
    pub users: UsersServiceClient<Channel>,
}

/// Контур API: прод или песочница
#[derive(Clone, Copy)]
pub enum EndPoint {
    /// Продовый контур
    Prod,
    /// Песочница
    Sandbox,
}

impl EndPoint {
    fn url(&self) -> &'static str {
        match self {
            EndPoint::Prod => "https://invest-public-api.tinkoff.ru:443",
            EndPoint::Sandbox => "https://sandbox-invest-public-api.tinkoff.ru:443",
        }
    }
}

impl Client {
    /// Создаёт новый клиент для взаимодействия с T-Invest API
    pub async fn new(auth_token: String, end_point: EndPoint) -> Self {
        // Создаём канал
        let tls_config = ClientTlsConfig::new().with_native_roots();
        let channel = Endpoint::from_shared(end_point.url())
            .unwrap()
            .tls_config(tls_config)
            .unwrap()
            .connect()
            .await
            .unwrap();

        // Создаём всех клиентов поверх одного канала
        Self {
            auth_token,
            instruments: InstrumentsServiceClient::new(channel.clone()),
            market_data: MarketDataServiceClient::new(channel.clone()),
            market_data_stream: MarketDataStreamServiceClient::new(channel.clone()),
            operations: OperationsServiceClient::new(channel.clone()),
            operations_stream: OperationsStreamServiceClient::new(channel.clone()),
            orders: OrdersServiceClient::new(channel.clone()),
            orders_stream: OrdersStreamServiceClient::new(channel.clone()),
            sandbox: SandboxServiceClient::new(channel.clone()),
            signal: SignalServiceClient::new(channel.clone()),
            stop_orders: StopOrdersServiceClient::new(channel.clone()),
            users: UsersServiceClient::new(channel),
        }
    }

    /// Создаёт новый запрос с добавлением токена авторизации в метаданные
    pub fn new_request<T>(&self, message: T) -> Request<T> {
        let mut request = Request::new(message);
        request
            .metadata_mut()
            .insert("authorization", format!("Bearer {}", self.auth_token).parse().unwrap());
        request
    }
}
