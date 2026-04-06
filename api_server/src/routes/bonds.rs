use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use t_invest_api_rust::decimal::money_value_to_decimal;
use t_invest_api_rust::proto::{InstrumentIdType, InstrumentRequest, InstrumentType, FindInstrumentRequest};

#[derive(Serialize)]
pub struct BondInfo {
    pub name: String,
    pub ticker: String,
    pub isin: String,
    pub figi: String,
    pub currency: String,
    pub nominal: Option<String>,
    pub aci_value: Option<String>,
    pub coupon_quantity_per_year: i32,
    pub maturity_date: Option<String>,
    pub placement_date: Option<String>,
    pub country_of_risk_name: String,
    pub sector: String,
    pub lot: i32,
    pub exchange: String,
    pub short_enabled: bool,
    pub buy_available: bool,
    pub sell_available: bool,
}

pub async fn get_bond_info(
    AuthUser(user_id): AuthUser,
    State(state): State<AppState>,
    Path(isin): Path<String>,
) -> Result<Json<BondInfo>, AppError> {
    // Get user's T-Invest token
    let row: Option<(Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT tinvest_token, tinvest_account_id, tinvest_endpoint FROM app_user WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

    let (token, _account_id, endpoint) = match row {
        Some((Some(t), Some(a), e)) => (t, a, e.unwrap_or_else(|| "sandbox".to_string())),
        _ => return Err(AppError::BadRequest("T-Invest not connected".into())),
    };

    let ep = match endpoint.as_str() {
        "production" => t_invest_api_rust::EndPoint::Prod,
        _ => t_invest_api_rust::EndPoint::Sandbox,
    };

    let mut client = t_invest_api_rust::Client::try_new(token, ep)
        .await
        .map_err(|e| AppError::Internal(format!("T-Invest connection failed: {e}")))?;

    // Search for the instrument by ISIN/ticker
    let search_resp = client
        .instruments
        .find_instrument(FindInstrumentRequest {
            query: isin.clone(),
            instrument_kind: Some(InstrumentType::Bond.into()),
            api_trade_available_flag: None,
        })
        .await
        .map_err(|e| AppError::Internal(format!("Find instrument failed: {e}")))?
        .into_inner();

    let found = search_resp
        .instruments
        .first()
        .ok_or_else(|| AppError::NotFound)?;

    // Get full bond info by FIGI
    let bond_resp = client
        .instruments
        .bond_by(InstrumentRequest {
            id_type: InstrumentIdType::Figi.into(),
            class_code: None,
            id: found.figi.clone(),
        })
        .await
        .map_err(|e| AppError::Internal(format!("Bond info failed: {e}")))?
        .into_inner();

    let bond = bond_resp
        .instrument
        .ok_or_else(|| AppError::Internal("Empty bond response".into()))?;

    let format_ts = |secs: i64| -> String {
        let dt = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        dt.format("%d.%m.%Y").to_string()
    };

    Ok(Json(BondInfo {
        name: bond.name,
        ticker: bond.ticker,
        isin: bond.isin,
        figi: bond.figi,
        currency: bond.currency.to_uppercase(),
        nominal: bond.nominal.as_ref().map(|n| money_value_to_decimal(n).to_string()),
        aci_value: bond.aci_value.as_ref().map(|a| money_value_to_decimal(a).to_string()),
        coupon_quantity_per_year: bond.coupon_quantity_per_year,
        maturity_date: bond.maturity_date.as_ref().map(|d| format_ts(d.seconds)),
        placement_date: bond.placement_date.as_ref().map(|d| format_ts(d.seconds)),
        country_of_risk_name: bond.country_of_risk_name,
        sector: bond.sector,
        lot: bond.lot,
        exchange: bond.exchange,
        short_enabled: bond.short_enabled_flag,
        buy_available: bond.buy_available_flag,
        sell_available: bond.sell_available_flag,
    }))
}
