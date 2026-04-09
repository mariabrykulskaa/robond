//! REST API сервер для управления портфелями облигаций.

mod auth;
mod config;
mod error;
mod routes;
mod state;

use config::ApiConfig;
use portfolio::PortfolioClient;
use sqlx::postgres::PgPoolOptions;
use state::AppState;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_server=debug,tower_http=debug".into()),
        )
        .init();

    let api_config = ApiConfig::from_env();
    let db_config = history_market_data::DbConfig::from_env().expect("DB config from env");

    let pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .connect(&db_config.database_url())
        .await
        .expect("Failed to connect to database");

    let portfolio_client = PortfolioClient::new(pool.clone());
    portfolio_client
        .run_migrations()
        .await
        .expect("Failed to run migrations");

    let state = AppState {
        pool,
        portfolio_client,
        jwt_secret: api_config.jwt_secret,
    };

    let app = routes::build_router(state);

    let listener = TcpListener::bind(&api_config.listen_addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("API server listening on {}", api_config.listen_addr);
    axum::serve(listener, app).await.expect("Server error");
}
