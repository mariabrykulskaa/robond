use axum::extract::{Path, State};
use axum::Json;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetCashRequest {
    pub amount: Decimal,
    pub currency: String,
}

#[derive(Serialize)]
pub struct CashResponse {
    pub amount: Decimal,
}

pub async fn get(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<CashResponse>, AppError> {
    state.portfolio_client.get_portfolio_for_user(user_id, portfolio_id).await?;
    let amount = state.portfolio_client.get_cash(portfolio_id).await?;
    Ok(Json(CashResponse { amount }))
}

pub async fn set(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
    Json(req): Json<SetCashRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.portfolio_client.get_portfolio_for_user(user_id, portfolio_id).await?;
    let cash = state.portfolio_client.set_cash(portfolio_id, req.amount, &req.currency).await?;
    Ok(Json(serde_json::to_value(cash).unwrap()))
}
