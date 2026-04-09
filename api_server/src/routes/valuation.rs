use axum::extract::{Path, State};
use axum::Json;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use t_invest_api_rust::decimal::{money_value_to_decimal, quotation_to_decimal};
use t_invest_api_rust::proto::{
    FindInstrumentRequest, GetLastPricesRequest, InstrumentIdType, InstrumentRequest,
    InstrumentType,
};

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

    let (token, _account_id, endpoint) = match tinvest {
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

    // Resolve each ISIN to bond info (name, nominal, aci, ticker_classCode)
    struct BondMeta {
        name: String,
        ticker: String,
        nominal: Decimal,
        aci_value: Decimal,
    }

    let mut meta: HashMap<String, BondMeta> = HashMap::new();
    let mut instrument_ids: Vec<String> = Vec::new();

    for h in &holdings {
        let search = client
            .instruments
            .find_instrument(FindInstrumentRequest {
                query: h.isin.clone(),
                instrument_kind: Some(InstrumentType::Bond.into()),
                api_trade_available_flag: None,
            })
            .await;

        let found = match search {
            Ok(resp) => resp.into_inner().instruments.into_iter().next(),
            Err(_) => None,
        };

        if let Some(found) = found {
            let bond_resp = client
                .instruments
                .bond_by(InstrumentRequest {
                    id_type: InstrumentIdType::Figi.into(),
                    class_code: None,
                    id: found.figi.clone(),
                })
                .await;

            if let Ok(resp) = bond_resp {
                if let Some(bond) = resp.into_inner().instrument {
                    let nominal = bond
                        .nominal
                        .as_ref()
                        .map(money_value_to_decimal)
                        .unwrap_or(Decimal::from(1000));
                    let aci = bond
                        .aci_value
                        .as_ref()
                        .map(money_value_to_decimal)
                        .unwrap_or(Decimal::ZERO);

                    instrument_ids.push(format!("{}_{}", bond.ticker, bond.class_code));

                    meta.insert(
                        h.isin.clone(),
                        BondMeta {
                            name: bond.name,
                            ticker: bond.ticker,
                            nominal,
                            aci_value: aci,
                        },
                    );
                }
            }
        }
    }

    // Batch-fetch last prices (default type = any last known price)
    let last_prices = if !instrument_ids.is_empty() {
        let request = GetLastPricesRequest {
            instrument_id: instrument_ids,
            ..GetLastPricesRequest::default()
        };
        client
            .market_data
            .get_last_prices(request)
            .await
            .ok()
            .map(|r| r.into_inner().last_prices)
            .unwrap_or_default()
    } else {
        vec![]
    };

    // ticker → price in points
    let mut ticker_points: HashMap<String, Decimal> = HashMap::new();
    for lp in &last_prices {
        if let Some(ref price) = lp.price {
            ticker_points.insert(lp.ticker.clone(), quotation_to_decimal(price.clone()));
        }
    }

    // Assemble result
    let mut total_bonds = Decimal::ZERO;
    let mut result_holdings = Vec::with_capacity(holdings.len());

    for h in &holdings {
        let (name, price_rub, estimated) = if let Some(bm) = meta.get(&h.isin) {
            if let Some(pts) = ticker_points.get(&bm.ticker) {
                let price = *pts / Decimal::from(100) * bm.nominal + bm.aci_value;
                (bm.name.clone(), price, false)
            } else {
                // No market price — fallback to nominal + ACI
                let price = bm.nominal + bm.aci_value;
                (bm.name.clone(), price, true)
            }
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
