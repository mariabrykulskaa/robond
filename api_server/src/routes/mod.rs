pub mod bonds;
pub mod cash;
pub mod holdings;
pub mod portfolio;
pub mod snapshots;
pub mod strategy;
pub mod tinvest;
pub mod valuation;

use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::handlers as auth_handlers;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route("/signup", post(auth_handlers::signup))
        .route("/login", post(auth_handlers::login))
        .route("/refresh", post(auth_handlers::refresh));

    let portfolio_routes = Router::new()
        .route("/", get(portfolio::list).post(portfolio::create))
        .route("/{id}", get(portfolio::get))
        .route("/{id}/holdings", get(holdings::list))
        .route("/{id}/holdings/{isin}", put(holdings::set))
        .route("/{id}/cash", get(cash::get).put(cash::set))
        .route("/{id}/snapshots", get(snapshots::list))
        .route("/{id}/return", get(snapshots::total_return))
        .route("/{id}/value", get(valuation::get_portfolio_value))
        .route(
            "/{id}/strategy",
            put(strategy::set_strategy).delete(strategy::clear_strategy),
        )
        .route("/{id}/strategy/run", post(strategy::run_strategy));

    let tinvest_routes = Router::new()
        .route("/accounts", post(tinvest::fetch_accounts))
        .route("/connect", post(tinvest::connect))
        .route("/status", get(tinvest::status))
        .route("/disconnect", delete(tinvest::disconnect))
        .route("/import/{portfolio_id}", post(tinvest::import_portfolio));

    let strategy_routes = Router::new().route("/", get(strategy::list_strategies));

    let bond_routes = Router::new().route("/{isin}", get(bonds::get_bond_info));

    Router::new()
        .nest("/api/auth", auth_routes)
        .nest("/api/portfolios", portfolio_routes)
        .nest("/api/tinvest", tinvest_routes)
        .nest("/api/strategies", strategy_routes)
        .nest("/api/bonds", bond_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
