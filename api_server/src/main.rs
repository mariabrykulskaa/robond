//! REST API сервер для управления портфелями облигаций.

mod auth;
mod config;
mod error;
mod routes;
mod state;

use config::ApiConfig;
use chrono::Timelike;
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

    let app = routes::build_router(state.clone());

    // Background scheduler: execute pending strategy runs when exchange opens
    tokio::spawn(pending_strategy_scheduler(state));

    let listener = TcpListener::bind(&api_config.listen_addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("API server listening on {}", api_config.listen_addr);
    axum::serve(listener, app).await.expect("Server error");
}

/// Background task: every 60s, check if exchange is open and execute pending strategy runs.
/// Also runs daily auto-execution at ~10:15 MSK for all portfolios with active strategies.
async fn pending_strategy_scheduler(state: state::AppState) {
    let mut last_daily_run: Option<chrono::NaiveDate> = None;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        if !routes::strategy::is_exchange_open() {
            continue;
        }

        let moscow_now = chrono::Utc::now() + chrono::Duration::hours(3);
        let today = moscow_now.date_naive();
        let t = moscow_now.time();
        let time_mins = t.hour() * 60 + t.minute();

        // Daily auto-run: at 10:15+ MSK, once per day
        let should_daily_run = time_mins >= 10 * 60 + 15
            && last_daily_run.map_or(true, |d| d < today);

        if should_daily_run {
            last_daily_run = Some(today);
            tracing::info!("Scheduler: daily strategy run triggered at {}", moscow_now.format("%H:%M MSK"));

            // Get all portfolios with active strategy + T-Invest connected
            let all_rows: Vec<(i64, i64)> = match sqlx::query_as(
                "SELECT p.id, p.user_id FROM portfolio p WHERE p.strategy_name IS NOT NULL AND p.tinvest_token IS NOT NULL"
            )
            .fetch_all(&state.pool)
            .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("Scheduler: failed to query daily portfolios: {e}");
                    vec![]
                }
            };

            for (portfolio_id, user_id) in all_rows {
                tracing::info!("Scheduler: daily run for portfolio {portfolio_id}");
                match routes::strategy::execute_strategy(
                    &state.pool,
                    &state.portfolio_client,
                    portfolio_id,
                    user_id,
                )
                .await
                {
                    Ok(result) => {
                        tracing::info!("Scheduler: portfolio {portfolio_id} done — {}", result.message);
                    }
                    Err(e) => {
                        tracing::error!("Scheduler: portfolio {portfolio_id} failed — {e:?}");
                    }
                }
            }
            continue; // daily run covers pending too
        }

        // Pending runs (queued outside trading hours)
        let rows: Vec<(i64, i64)> = match sqlx::query_as(
            "SELECT p.id, p.user_id FROM portfolio p WHERE p.pending_strategy_run = TRUE AND p.strategy_name IS NOT NULL AND p.tinvest_token IS NOT NULL"
        )
        .fetch_all(&state.pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Scheduler: failed to query pending runs: {e}");
                continue;
            }
        };

        for (portfolio_id, user_id) in rows {
            tracing::info!("Scheduler: executing pending strategy for portfolio {portfolio_id}");
            match routes::strategy::execute_strategy(
                &state.pool,
                &state.portfolio_client,
                portfolio_id,
                user_id,
            )
            .await
            {
                Ok(result) => {
                    tracing::info!("Scheduler: portfolio {portfolio_id} done — {}", result.message);
                }
                Err(e) => {
                    tracing::error!("Scheduler: portfolio {portfolio_id} failed — {e:?}");
                    let _ = sqlx::query("UPDATE portfolio SET pending_strategy_run = FALSE WHERE id = $1")
                        .bind(portfolio_id)
                        .execute(&state.pool)
                        .await;
                }
            }
        }
    }
}
