use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreatePortfolioRequest {
    pub name: String,
}

pub async fn list(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let portfolios = state.portfolio_client.list_portfolios_for_user(user_id).await?;
    Ok(Json(serde_json::to_value(portfolios).unwrap()))
}

pub async fn create(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreatePortfolioRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let portfolio = state
        .portfolio_client
        .create_portfolio_for_user(user_id, &req.name)
        .await?;
    Ok(Json(serde_json::to_value(portfolio).unwrap()))
}

pub async fn get(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let portfolio = state.portfolio_client.get_portfolio_for_user(user_id, id).await?;
    Ok(Json(serde_json::to_value(portfolio).unwrap()))
}
