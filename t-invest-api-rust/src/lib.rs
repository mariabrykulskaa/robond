//! Этот крейт предоставляет удобный интерфейс для взаимодействия с T-Invest API по протоколу gRPC.
//!
//! # TODO
//!
//! - Добавить комментарии к полям структуры `Client`
//! - Добавить добавление заголовков x-tracking-id и AppName в интерсептор
//! - Зарелизить крейт на crates.io
//!
//! # Пример
//!
//! ```no_run
//! use std::fs;
//! use t_invest_api_rust::{
//!     Client, EndPoint, Request,
//!     proto::{GetInfoRequest, GetLastPricesRequest, InstrumentStatus, LastPriceType},
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let authorization_token = fs::read_to_string("authorization_token.txt").unwrap();
//!     let mut client = Client::try_new(authorization_token, EndPoint::Prod).await.unwrap();
//!
//!     let request = GetInfoRequest {};
//!     let request = Request::new(request);
//!     let response = client.users.get_info(request).await.unwrap().into_inner();
//!     dbg!(&response);
//!
//!     let mut request = GetLastPricesRequest {
//!         instrument_id: vec!["T_TQBR".to_string()],
//!         ..GetLastPricesRequest::default()
//!     };
//!     request.set_last_price_type(LastPriceType::LastPriceExchange);
//!     request.set_instrument_status(InstrumentStatus::Base);
//!     let request = Request::new(request);
//!     let response = client.market_data.get_last_prices(request).await.unwrap().into_inner();
//!     dbg!(&response);
//! }
//! ```

#![warn(missing_docs)]

#[allow(missing_docs)]
#[allow(clippy::all)]
/// Код, сгенерированный из protobuf-контракта T-Invest API
pub mod proto {
    tonic::include_proto!("tinkoff.public.invest.api.contract.v1");
}

mod error;

pub use error::{Error, Result};

pub use tonic::Request;

use tonic::{
    Status,
    metadata::{Ascii, MetadataValue},
    service::{Interceptor, interceptor::InterceptedService},
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
#[derive(Clone, Debug)]
pub struct Client {
    #[allow(missing_docs)]
    pub instruments: InstrumentsServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub market_data: MarketDataServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub market_data_stream: MarketDataStreamServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub operations: OperationsServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub operations_stream: OperationsStreamServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub orders: OrdersServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub orders_stream: OrdersStreamServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub sandbox: SandboxServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub signal: SignalServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub stop_orders: StopOrdersServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
    #[allow(missing_docs)]
    pub users: UsersServiceClient<InterceptedService<Channel, AuthorizationInterceptor>>,
}

/// Контур API: прод или песочница
#[derive(Clone, Copy, PartialEq, Eq)]
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

/// Интерсептор gRPC-клиента, добавляющий к каждому запросу заголовок `Authorization: Bearer <token>`.
#[derive(Clone)]
pub struct AuthorizationInterceptor {
    authorization_header_value: MetadataValue<Ascii>,
}

impl Interceptor for AuthorizationInterceptor {
    fn call(&mut self, mut request: Request<()>) -> std::result::Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.authorization_header_value.clone());
        Ok(request)
    }
}

impl Client {
    /// Создаёт новый клиент для взаимодействия с T-Invest API
    pub async fn try_new(authorization_token: String, end_point: EndPoint) -> Result<Self> {
        let authorization_header_value: MetadataValue<Ascii> = format!("Bearer {}", authorization_token).parse()?;
        let interceptor = AuthorizationInterceptor {
            authorization_header_value,
        };

        // Создаём канал
        let tls_config = ClientTlsConfig::new().with_native_roots();
        let channel = Endpoint::from_shared(end_point.url())?
            .tls_config(tls_config)?
            .connect()
            .await?;

        // Создаём всех клиентов поверх одного канала
        Ok(Self {
            instruments: InstrumentsServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            market_data: MarketDataServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            market_data_stream: MarketDataStreamServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            operations: OperationsServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            operations_stream: OperationsStreamServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            orders: OrdersServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            orders_stream: OrdersStreamServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            sandbox: SandboxServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            signal: SignalServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            stop_orders: StopOrdersServiceClient::with_interceptor(channel.clone(), interceptor.clone()),
            users: UsersServiceClient::with_interceptor(channel, interceptor),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_authorization_token_characters() {
        let error = Client::try_new("\n".to_string(), EndPoint::Prod).await.unwrap_err();
        assert!(matches!(error, Error::InvalidAuthorizationTokenCharacters(_)));
    }
}
