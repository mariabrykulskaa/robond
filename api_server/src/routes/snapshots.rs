use axum::extract::{Path, State};
use axum::Json;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct TotalReturnResponse {
    pub total_return: Option<Decimal>,
}

pub async fn list(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;
    let snapshots = state.portfolio_client.get_snapshots(portfolio_id).await?;
    Ok(Json(serde_json::to_value(snapshots).unwrap()))
}

pub async fn total_return(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<TotalReturnResponse>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;
    let ret = state.portfolio_client.compute_total_return(portfolio_id).await?;
    Ok(Json(TotalReturnResponse { total_return: ret }))
}
