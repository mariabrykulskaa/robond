use axum::extract::{Path, State};
use axum::Json;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use t_invest_api_rust::decimal::{money_value_to_decimal, quotation_to_decimal};
use t_invest_api_rust::proto::PortfolioRequest;

#[derive(Serialize)]
pub struct HoldingValue {
    pub isin: String,
    pub name: String,
    pub quantity: i64,
    pub price: String,
    pub value: String,
    /// true if price is estimated (nominal + ACI) because market price is unavailable
    pub estimated: bool,
}

#[derive(Serialize)]
pub struct PortfolioValue {
    pub holdings: Vec<HoldingValue>,
    pub bonds_value: String,
    pub cash: String,
    pub total_value: String,
}

pub async fn get_portfolio_value(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(portfolio_id): Path<i64>,
) -> Result<Json<PortfolioValue>, AppError> {
    state
        .portfolio_client
        .get_portfolio_for_user(user_id, portfolio_id)
        .await?;

    let holdings = state.portfolio_client.get_holdings(portfolio_id).await?;
    let cash = state.portfolio_client.get_cash(portfolio_id).await?;

    if holdings.is_empty() {
        return Ok(Json(PortfolioValue {
            holdings: vec![],
            bonds_value: "0".into(),
            cash: cash.to_string(),
            total_value: cash.to_string(),
        }));
    }

    // Get T-Invest token from portfolio
    let tinvest = super::tinvest::get_portfolio_tinvest(&state.pool, user_id, portfolio_id).await;

    let (token, account_id, endpoint) = match tinvest {
        Ok(creds) => creds,
        Err(_) => {
            // No T-Invest — return holdings without prices
            let hv: Vec<HoldingValue> = holdings
                .iter()
                .map(|h| HoldingValue {
                    isin: h.isin.clone(),
                    name: h.isin.clone(),
                    quantity: h.quantity,
                    price: "0".into(),
                    value: "0".into(),
                    estimated: true,
                })
                .collect();
            return Ok(Json(PortfolioValue {
                holdings: hv,
                bonds_value: "0".into(),
                cash: cash.to_string(),
                total_value: cash.to_string(),
            }));
        }
    };

    let ep = match endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(token, ep)
        .await
        .map_err(|e| AppError::Internal(format!("T-Invest connection failed: {e}")))?;

    // Use GetPortfolio — returns positions with current prices directly
    let portfolio_resp = client
        .operations
        .get_portfolio(PortfolioRequest {
            account_id: account_id.clone(),
            currency: None,
        })
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get portfolio: {e}")))?
        .into_inner();

    // Build ticker → position data from T-Invest portfolio
    struct PositionData {
        name: String,
        price_per_one: Decimal,
    }

    let mut position_map: HashMap<String, PositionData> = HashMap::new();
    for pos in &portfolio_resp.positions {
        if pos.instrument_type != "bond" {
            continue;
        }
        let _qty = pos
            .quantity
            .as_ref()
            .map(|q| quotation_to_decimal(q.clone()))
            .unwrap_or(Decimal::ZERO);
        let price = pos
            .current_price
            .as_ref()
            .map(money_value_to_decimal)
            .unwrap_or(Decimal::ZERO);
        let nkd = pos
            .current_nkd
            .as_ref()
            .map(money_value_to_decimal)
            .unwrap_or(Decimal::ZERO);

        let ticker = &pos.ticker;
        if !ticker.is_empty() {
            position_map.insert(
                ticker.clone(),
                PositionData {
                    name: ticker.clone(),
                    price_per_one: price + nkd,
                },
            );
        }
        // Also index by figi for fallback matching
        if !pos.figi.is_empty() {
            position_map.insert(
                pos.figi.clone(),
                PositionData {
                    name: ticker.clone(),
                    price_per_one: price + nkd,
                },
            );
        }
    }

    // Assemble result matching DB holdings with T-Invest position data
    let mut total_bonds = Decimal::ZERO;
    let mut result_holdings = Vec::with_capacity(holdings.len());

    for h in &holdings {
        let (name, price_rub, estimated) = if let Some(pd) = position_map.get(&h.isin) {
            (pd.name.clone(), pd.price_per_one, false)
        } else {
            (h.isin.clone(), Decimal::ZERO, true)
        };

        let val = price_rub * Decimal::from(h.quantity);
        total_bonds += val;

        result_holdings.push(HoldingValue {
            isin: h.isin.clone(),
            name,
            quantity: h.quantity,
            price: price_rub.round_dp(2).to_string(),
            value: val.round_dp(2).to_string(),
            estimated,
        });
    }

    let total = total_bonds + cash;

    Ok(Json(PortfolioValue {
        holdings: result_holdings,
        bonds_value: total_bonds.round_dp(2).to_string(),
        cash: cash.round_dp(2).to_string(),
        total_value: total.round_dp(2).to_string(),
    }))
}
