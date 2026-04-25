use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct StrategyInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct SetStrategyRequest {
    pub strategy_name: String,
}

#[derive(Serialize)]
pub struct RunResult {
    pub orders_count: usize,
    pub message: String,
}

const VALID_STRATEGIES: &[&str] = &["diversified_short_duration", "high_yield_short", "yield_maximizer"];

pub async fn list_strategies() -> Json<Vec<StrategyInfo>> {
    Json(vec![
        StrategyInfo {
            id: "diversified_short_duration".into(),
            name: "Консервативная".into(),
            description: "Диверсифицированный портфель коротких облигаций (3–18 мес.) с дисконтом к номиналу. Не более 15 % на одну бумагу, автоматический стоп-лосс при падении цены ниже 70 %. Низкий риск, стабильная доходность.".into(),
        },
        StrategyInfo {
            id: "high_yield_short".into(),
            name: "Агрессивная".into(),
            description: "Максимальная доходность на коротких облигациях (до 1 года) с XIRR ≥ 10 %. Не более 8 % на одну бумагу, стоп-лосс при падении ниже 70 %. Высокая доходность, повышенный риск.".into(),
        },
        StrategyInfo {
            id: "yield_maximizer".into(),
            name: "Умеренная".into(),
            description: "Сбалансированный подход: покупка облигаций с оптимальным XIRR, не более 5 % на бумагу. Удержание до погашения, динамическое снижение порога при избытке кэша. Баланс между доходностью и контролем рисков.".into(),
        },
    ])
}

pub async fn set_strategy(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
    Json(req): Json<SetStrategyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !VALID_STRATEGIES.contains(&req.strategy_name.as_str()) {
        return Err(AppError::BadRequest(format!(
            "unknown strategy '{}'. Valid: {:?}",
            req.strategy_name, VALID_STRATEGIES
        )));
    }

    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let portfolio = state
        .portfolio_client
        .set_strategy(portfolio_id, &req.strategy_name)
        .await?;

    Ok(Json(serde_json::to_value(portfolio).unwrap()))
}

pub async fn clear_strategy(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let portfolio = state.portfolio_client.clear_strategy(portfolio_id).await?;
    Ok(Json(serde_json::to_value(portfolio).unwrap()))
}

pub async fn run_strategy(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<RunResult>, AppError> {
    // Get portfolio and verify ownership
    let portfolio = state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let strategy_name = portfolio
        .strategy_name
        .ok_or_else(|| AppError::BadRequest("no strategy assigned to this portfolio".into()))?;

    // Get T-Invest credentials
    let row: Option<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT tinvest_token, tinvest_account_id, tinvest_endpoint FROM app_user WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

    let (token, account_id, endpoint) = match row {
        Some((Some(t), Some(a), e)) => (t, a, e.unwrap_or_else(|| "sandbox".to_string())),
        _ => return Err(AppError::BadRequest("T-Invest not connected".into())),
    };

    // Connect to T-Invest
    let ep = match endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(token, ep)
        .await
        .map_err(|e| AppError::Internal(format!("T-Invest connection failed: {e}")))?;

    // Run the strategy via live_engine
    match strategy_name.as_str() {
        "diversified_short_duration" => {
            let strat = trading_strategies::diversified_short_duration::DiversifiedShortDurationStrategy::default();
            live_engine::run(&account_id, &mut client, strat).await;
        }
        "high_yield_short" => {
            let strat = trading_strategies::high_yield_short::HighYieldShortStrategy::default();
            live_engine::run(&account_id, &mut client, strat).await;
        }
        "yield_maximizer" => {
            let strat = trading_strategies::yield_maximizer::YieldMaximizerStrategy::default();
            live_engine::run(&account_id, &mut client, strat).await;
        }
        _ => return Err(AppError::BadRequest("unknown strategy".into())),
    }

    // Auto-import: sync portfolio from T-Invest after strategy execution
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

    Ok(Json(RunResult {
        orders_count: holdings_imported,
        message: format!(
            "Strategy '{}' executed. Imported {} holdings, cash: {} RUB",
            strategy_name, holdings_imported, cash
        ),
    }))
}
