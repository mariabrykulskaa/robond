use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct FetchAccountsRequest {
    pub token: String,
    pub endpoint: String,
    pub initial_amount: Option<i64>,
}

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub account_type: String,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub token: String,
    pub account_id: String,
    pub endpoint: String,
}

#[derive(Serialize)]
pub struct TInvestStatus {
    pub connected: bool,
    pub account_id: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub holdings_imported: usize,
    pub cash_rub: String,
}

/// Helper: get T-Invest credentials from a portfolio row.
pub async fn get_portfolio_tinvest(
    pool: &sqlx::PgPool,
    user_id: i64,
    portfolio_id: i64,
) -> Result<(String, String, String), AppError> {
    let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT tinvest_token, tinvest_account_id, tinvest_endpoint FROM portfolio WHERE id = $1 AND user_id = $2",
    )
    .bind(portfolio_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    match row {
        Some((Some(t), Some(a), e)) => Ok((t, a, e.unwrap_or_else(|| "sandbox".to_string()))),
        Some(_) => Err(AppError::BadRequest("T-Invest not connected for this portfolio".into())),
        None => Err(AppError::NotFound),
    }
}

/// Fetch available accounts for a given token (no saving yet).
pub async fn fetch_accounts(
    AuthUser(_user_id): AuthUser,
    Json(req): Json<FetchAccountsRequest>,
) -> Result<Json<Vec<AccountInfo>>, AppError> {
    if req.token.is_empty() {
        return Err(AppError::BadRequest("token is required".into()));
    }

    let ep = match req.endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(req.token, ep)
        .await
        .map_err(|e| AppError::BadRequest(format!("Не удалось подключиться к T-Invest: {e}")))?;

    let is_sandbox = matches!(ep, t_invest_api_rust::EndPoint::Sandbox);

    if is_sandbox {
        // Always create a fresh sandbox account for each portfolio
        let new_acc = client
            .sandbox
            .open_sandbox_account(t_invest_api_rust::proto::OpenSandboxAccountRequest {
                name: Some("Sandbox".to_string()),
            })
            .await
            .map_err(|e| AppError::BadRequest(format!("Не удалось создать sandbox-счёт: {e}")))?
            .into_inner();

        // Top up the new sandbox account
        let amount = req.initial_amount.unwrap_or(1_000_000);
        client
            .sandbox
            .sandbox_pay_in(t_invest_api_rust::proto::SandboxPayInRequest {
                account_id: new_acc.account_id.clone(),
                amount: Some(t_invest_api_rust::proto::MoneyValue {
                    currency: "RUB".to_string(),
                    units: amount,
                    nano: 0,
                }),
            })
            .await
            .map_err(|e| AppError::BadRequest(format!("Не удалось пополнить sandbox: {e}")))?;

        tracing::info!("Created sandbox account {} with {} RUB", new_acc.account_id, amount);

        // Return only the newly created account
        return Ok(Json(vec![AccountInfo {
            id: new_acc.account_id,
            name: format!("Новый sandbox-счёт ({} ₽)", amount.to_string().as_str()
                .chars().rev().enumerate()
                .flat_map(|(i, c)| { if i > 0 && i % 3 == 0 { vec![' ', c] } else { vec![c] } })
                .collect::<Vec<_>>().into_iter().rev().collect::<String>()),
            account_type: "Sandbox".to_string(),
        }]));
    }

    let response = client
        .users
        .get_accounts(t_invest_api_rust::proto::GetAccountsRequest { status: None })
        .await
        .map_err(|e| AppError::BadRequest(format!("Не удалось получить счета: {e}")))?
        .into_inner();

    let accounts = response
        .accounts
        .into_iter()
        .map(|a| {
            let account_type = match a.r#type() {
                t_invest_api_rust::proto::AccountType::Tinkoff => "Брокерский счёт",
                t_invest_api_rust::proto::AccountType::TinkoffIis => "ИИС",
                t_invest_api_rust::proto::AccountType::InvestBox => "Инвесткопилка",
                _ => "Другой",
            };
            AccountInfo {
                id: a.id,
                name: if a.name.is_empty() {
                    account_type.to_string()
                } else {
                    a.name
                },
                account_type: account_type.to_string(),
            }
        })
        .collect();

    Ok(Json(accounts))
}

pub async fn connect(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
    Json(req): Json<ConnectRequest>,
) -> Result<Json<TInvestStatus>, AppError> {
    if req.token.is_empty() || req.account_id.is_empty() {
        return Err(AppError::BadRequest("token and account_id are required".into()));
    }

    // Verify portfolio ownership
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let endpoint = match req.endpoint.as_str() {
        "production" => "production",
        _ => "sandbox",
    };

    sqlx::query("UPDATE portfolio SET tinvest_token = $2, tinvest_account_id = $3, tinvest_endpoint = $4 WHERE id = $1 AND user_id = $5")
        .bind(portfolio_id)
        .bind(&req.token)
        .bind(&req.account_id)
        .bind(endpoint)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Clear old holdings and cash when switching to a new account
    sqlx::query("DELETE FROM portfolio_holding WHERE portfolio_id = $1")
        .bind(portfolio_id)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query("DELETE FROM portfolio_cash WHERE portfolio_id = $1")
        .bind(portfolio_id)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(TInvestStatus {
        connected: true,
        account_id: Some(req.account_id),
        endpoint: Some(endpoint.to_string()),
    }))
}

pub async fn status(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<TInvestStatus>, AppError> {
    let row: Option<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT tinvest_token, tinvest_account_id, tinvest_endpoint FROM portfolio WHERE id = $1 AND user_id = $2")
            .bind(portfolio_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

    match row {
        Some((Some(_token), Some(account_id), endpoint)) => Ok(Json(TInvestStatus {
            connected: true,
            account_id: Some(account_id),
            endpoint,
        })),
        _ => Ok(Json(TInvestStatus {
            connected: false,
            account_id: None,
            endpoint: None,
        })),
    }
}

pub async fn disconnect(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<TInvestStatus>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    sqlx::query(
        "UPDATE portfolio SET tinvest_token = NULL, tinvest_account_id = NULL, tinvest_endpoint = 'sandbox' WHERE id = $1 AND user_id = $2",
    )
    .bind(portfolio_id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(TInvestStatus {
        connected: false,
        account_id: None,
        endpoint: None,
    }))
}

pub async fn import_portfolio(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<ImportResult>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let (token, account_id, endpoint) =
        get_portfolio_tinvest(&state.pool, user_id, portfolio_id).await?;

    let ep = match endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(token, ep)
        .await
        .map_err(|e| AppError::Internal(format!("T-Invest connection failed: {e}")))?;

    let tinvest_portfolio = live_engine::get_portfolio(&mut client, &account_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get portfolio: {e}")))?;

    let mut holdings_imported = 0;
    for (isin, &quantity) in &tinvest_portfolio.bonds_count {
        if quantity > 0 {
            state.portfolio_client.set_holding(portfolio_id, isin, quantity).await?;
            holdings_imported += 1;
        }
    }

    let cash = tinvest_portfolio.free_money;
    state.portfolio_client.set_cash(portfolio_id, cash, "RUB").await?;

    Ok(Json(ImportResult {
        holdings_imported,
        cash_rub: cash.to_string(),
    }))
}
