use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetHoldingRequest {
    pub quantity: i64,
}

pub async fn list(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify ownership
    state.portfolio_client.get_portfolio_for_user(user_id, portfolio_id).await?;
    let holdings = state.portfolio_client.get_holdings(portfolio_id).await?;
    Ok(Json(serde_json::to_value(holdings).unwrap()))
}

pub async fn set(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path((portfolio_id, isin)): Path<(i64, String)>,
    Json(req): Json<SetHoldingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.portfolio_client.get_portfolio_for_user(user_id, portfolio_id).await?;
    let holding = state.portfolio_client.set_holding(portfolio_id, &isin, req.quantity).await?;
    Ok(Json(serde_json::to_value(holding).unwrap()))
}
