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

pub async fn delete(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Close sandbox account if connected
    if let Ok((token, account_id, endpoint)) =
        super::tinvest::get_portfolio_tinvest(&state.pool, user_id, portfolio_id).await
    {
        if endpoint == "sandbox" {
            let ep = t_invest_api_rust::EndPoint::Sandbox;
            if let Ok(mut client) = t_invest_api_rust::Client::try_new(token, ep).await {
                let _ = client
                    .sandbox
                    .close_sandbox_account(t_invest_api_rust::proto::CloseSandboxAccountRequest {
                        account_id,
                    })
                    .await;
            }
        }
    }

    state
        .portfolio_client
        .delete_portfolio_for_user(user_id, portfolio_id)
        .await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
