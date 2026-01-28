//! Клиент для взаимодействия с Tinkoff Invest API
//!
//! Этот крейт предоставляет удобный интерфейс для взаимодействия с Tinkoff Invest API через gRPC.
//!
//! # TODO
//!
//! - Провести рефакторинг кода
//! - Добавить поддержку настройки endpoint (для песочницы и продакшена)
//! - Улучшить обработку ошибок (заменить unwrap на Result)
//!
//! # Пример
//!
//! ```no_run
//! use broker_api::{InvestApiClient, t_invest::{self, GetInfoRequest, GetLastPricesRequest}};
//! use std::fs;
//!
//! #[tokio::main]
//! async fn main() {
//!     let token = fs::read_to_string("../../tokens/readonly_token.txt").unwrap();
//!     let mut client = InvestApiClient::new(token).await;
//!     
//!     let request = client.new_request(GetInfoRequest {});
//!     let response = client.users.get_info(request).await.unwrap().into_inner();
//!     dbg!(&response);
//!
//!     let mut request = GetLastPricesRequest {
//!         figi: Vec::new(),
//!         instrument_id: vec!["T_TQBR".to_string()],
//!         last_price_type: 0,
//!         instrument_status: None,
//!     };
//!     request.set_last_price_type(t_invest::LastPriceType::LastPriceExchange);
//!     request.set_instrument_status(t_invest::InstrumentStatus::Base);
//!     let request = client.new_request(request);
//!     let response = client.market_data.get_last_prices(request).await.unwrap().into_inner();
//!     dbg!(&response);
//! }
//! ```

#![allow(clippy::all)]
pub mod t_invest {
    tonic::include_proto!("tinkoff.public.invest.api.contract.v1");
}

use tonic::{
    Request,
    transport::{Channel, ClientTlsConfig, Endpoint},
};

// Подключаем все сервисы
use t_invest::{
    instruments_service_client::InstrumentsServiceClient,
    market_data_service_client::MarketDataServiceClient,
    market_data_stream_service_client::MarketDataStreamServiceClient,
    operations_service_client::OperationsServiceClient,
    operations_stream_service_client::OperationsStreamServiceClient,
    orders_service_client::OrdersServiceClient,
    orders_stream_service_client::OrdersStreamServiceClient,
    sandbox_service_client::SandboxServiceClient, signal_service_client::SignalServiceClient,
    stop_orders_service_client::StopOrdersServiceClient, users_service_client::UsersServiceClient,
};

/// Клиент для взаимодействия с t invest api
#[derive(Clone)]
pub struct InvestApiClient {
    auth_token: String,
    pub users: UsersServiceClient<Channel>,
    pub orders: OrdersServiceClient<Channel>,
    pub signal: SignalServiceClient<Channel>,
    pub sandbox: SandboxServiceClient<Channel>,
    pub market_data: MarketDataServiceClient<Channel>,
    pub operations: OperationsServiceClient<Channel>,
    pub stop_orders: StopOrdersServiceClient<Channel>,
    pub instruments: InstrumentsServiceClient<Channel>,
    pub orders_stream: OrdersStreamServiceClient<Channel>,
    pub market_data_stream: MarketDataStreamServiceClient<Channel>,
    pub operations_stream: OperationsStreamServiceClient<Channel>,
}

impl InvestApiClient {
    /// Создаёт новый клиент для взаимодействия с t invest API
    pub async fn new(auth_token: String) -> Self {
        // Создаём канал
        let tls_config = ClientTlsConfig::new().with_native_roots();
        let channel = Endpoint::from_shared("https://invest-public-api.tinkoff.ru:443")
            .unwrap()
            .tls_config(tls_config)
            .unwrap()
            .connect()
            .await
            .unwrap();

        // Создаём всех клиентов поверх одного канала
        Self {
            auth_token,
            users: UsersServiceClient::new(channel.clone()),
            orders: OrdersServiceClient::new(channel.clone()),
            signal: SignalServiceClient::new(channel.clone()),
            sandbox: SandboxServiceClient::new(channel.clone()),
            market_data: MarketDataServiceClient::new(channel.clone()),
            operations: OperationsServiceClient::new(channel.clone()),
            stop_orders: StopOrdersServiceClient::new(channel.clone()),
            instruments: InstrumentsServiceClient::new(channel.clone()),
            orders_stream: OrdersStreamServiceClient::new(channel.clone()),
            market_data_stream: MarketDataStreamServiceClient::new(channel.clone()),
            operations_stream: OperationsStreamServiceClient::new(channel),
        }
    }

    /// Создаёт новый запрос с добавлением токена авторизации в метаданные
    pub fn new_request<T>(&self, message: T) -> Request<T> {
        let mut request = Request::new(message);
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.auth_token).parse().unwrap(),
        );
        request
    }
}
