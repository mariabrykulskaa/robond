use axum::extract::{Path, State};
use axum::Json;
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

/// Check if MOEX bond market is open (Mon-Fri, 10:00–18:50 Moscow time).
/// Returns Ok(()) if open or if endpoint is sandbox. Returns Err with message if closed.
fn check_exchange_open(endpoint: &str) -> Result<(), AppError> {
    if endpoint != "production" {
        return Ok(()); // sandbox works 24/7
    }

    let moscow_now = Utc::now() + chrono::Duration::hours(3); // UTC+3
    let weekday = moscow_now.weekday();
    let hour = moscow_now.hour();
    let minute = moscow_now.minute();
    let time_mins = hour * 60 + minute; // minutes since midnight

    let is_weekday = !matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun);
    let is_trading_hours = time_mins >= 10 * 60 && time_mins <= 18 * 60 + 50; // 10:00 – 18:50

    if is_weekday && is_trading_hours {
        Ok(())
    } else {
        let when = if !is_weekday {
            "Биржа не работает в выходные. Торги возобновятся в понедельник в 10:00 МСК."
        } else if time_mins < 10 * 60 {
            "Биржа ещё не открылась. Торги начинаются в 10:00 МСК."
        } else {
            "Биржа уже закрылась. Торги идут с 10:00 до 18:50 МСК."
        };
        Err(AppError::BadRequest(when.to_string()))
    }
}

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
) -> Result<Json<RunResult>, AppError> {
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

    // Get T-Invest credentials
    let (token, account_id, endpoint) =
        super::tinvest::get_portfolio_tinvest(&state.pool, user_id, portfolio_id).await?;

    // Check if exchange is open (skip for sandbox)
    check_exchange_open(&endpoint)?;

    let ep = match endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(token, ep)
        .await
        .map_err(|e| AppError::Internal(format!("T-Invest connection failed: {e}")))?;

    // Step 1: sell all current holdings
    let current = live_engine::get_portfolio(&mut client, &account_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get portfolio: {e}")))?;

    let ticker_to_info = live_engine::get_ticker_to_info(&mut client).await;

    let mut sell_orders = Vec::new();
    for (ticker, &count) in &current.bonds_count {
        if count > 0 {
            sell_orders.push(trading_strategies::MarketOrder {
                isin: ticker.clone(),
                order_type: trading_strategies::MarketOrderType::Sell,
                count,
            });
        }
    }

    if !sell_orders.is_empty() {
        live_engine::make_orders(&mut client, &sell_orders, &ticker_to_info, &account_id).await;
        // Small delay for orders to settle
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Step 2: save new strategy
    state
        .portfolio_client
        .set_strategy(portfolio_id, &req.strategy_name)
        .await?;

    // Step 3: run new strategy (it will buy based on fresh portfolio state)
    // Re-fetch portfolio after sells
    let mut client2 = t_invest_api_rust::Client::try_new(
        super::tinvest::get_portfolio_tinvest(&state.pool, user_id, portfolio_id)
            .await?
            .0,
        ep,
    )
    .await
    .map_err(|e| AppError::Internal(format!("T-Invest reconnection failed: {e}")))?;

    match req.strategy_name.as_str() {
        "diversified_short_duration" => {
            let strat = trading_strategies::diversified_short_duration::DiversifiedShortDurationStrategy::default();
            live_engine::run(&account_id, &mut client2, strat).await;
        }
        "high_yield_short" => {
            let strat = trading_strategies::high_yield_short::HighYieldShortStrategy::default();
            live_engine::run(&account_id, &mut client2, strat).await;
        }
        "yield_maximizer" => {
            let strat = trading_strategies::yield_maximizer::YieldMaximizerStrategy::default();
            live_engine::run(&account_id, &mut client2, strat).await;
        }
        _ => return Err(AppError::BadRequest("unknown strategy".into())),
    }

    // Step 4: sync portfolio from T-Invest
    let tinvest_portfolio = live_engine::get_portfolio(&mut client2, &account_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get portfolio: {e}")))?;

    // Clear old holdings from DB before importing
    sqlx::query("DELETE FROM portfolio_holding WHERE portfolio_id = $1")
        .bind(portfolio_id)
        .execute(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
            "Sold all positions, switched to '{}'. Bought {} new holdings, cash: {} RUB",
            req.strategy_name, holdings_imported, cash
        ),
    }))
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

    // Get T-Invest credentials from portfolio
    let (token, account_id, endpoint) =
        super::tinvest::get_portfolio_tinvest(&state.pool, user_id, portfolio_id).await?;

    // Check if exchange is open (skip for sandbox)
    check_exchange_open(&endpoint)?;

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
