use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use t_invest_api_rust::decimal::money_value_to_decimal;
use t_invest_api_rust::proto::{
    FindInstrumentRequest, GetBondCouponsRequest, InstrumentIdType, InstrumentRequest,
    InstrumentType,
};

fn none_if_empty(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s == "unknown" || s == "Unknown" {
        None
    } else {
        Some(s.to_string())
    }
}

fn coupon_type_name(ct: i32) -> Option<&'static str> {
    match ct {
        1 => Some("Постоянный"),
        2 => Some("Плавающий"),
        3 => Some("Дисконт"),
        4 => Some("Ипотечный"),
        5 => Some("Фиксированный"),
        6 => Some("Переменный"),
        7 => Some("Прочее"),
        _ => None,
    }
}

#[derive(Serialize)]
pub struct BondInfo {
    pub name: String,
    pub ticker: String,
    pub isin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nominal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aci_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_quantity_per_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maturity_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_of_risk_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coupon_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_coupon_date: Option<String>,
    pub floating_coupon: bool,
    pub amortization: bool,
    pub perpetual: bool,
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

    // Fetch next coupon info
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now_ts = prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: 0,
    };
    // Request coupons from now to +2 years
    let future_ts = prost_types::Timestamp {
        seconds: now.as_secs() as i64 + 365 * 2 * 86400,
        nanos: 0,
    };

    let (coupon_type, coupon_amount, next_coupon_date) = match client
        .instruments
        .get_bond_coupons(GetBondCouponsRequest {
            figi: String::new(),
            from: Some(now_ts),
            to: Some(future_ts),
            instrument_id: bond.figi.clone(),
        })
        .await
    {
        Ok(resp) => {
            let coupons = resp.into_inner().events;
            if let Some(next) = coupons.first() {
                let ct = coupon_type_name(next.coupon_type).map(String::from);
                let amount = next
                    .pay_one_bond
                    .as_ref()
                    .map(|m| format!("{} {}", money_value_to_decimal(m), m.currency.to_uppercase()));
                let date = next
                    .coupon_date
                    .as_ref()
                    .map(|d| format_ts(d.seconds));
                (ct, amount, date)
            } else {
                (None, None, None)
            }
        }
        Err(_) => (None, None, None),
    };

    let cqpy = if bond.coupon_quantity_per_year > 0 {
        Some(bond.coupon_quantity_per_year)
    } else {
        None
    };

    Ok(Json(BondInfo {
        name: bond.name,
        ticker: bond.ticker,
        isin: bond.isin,
        currency: none_if_empty(&bond.currency).map(|c| c.to_uppercase()),
        nominal: bond.nominal.as_ref().map(|n| money_value_to_decimal(n).to_string()),
        aci_value: bond.aci_value.as_ref().map(|a| money_value_to_decimal(a).to_string()),
        coupon_quantity_per_year: cqpy,
        maturity_date: bond.maturity_date.as_ref().map(|d| format_ts(d.seconds)),
        country_of_risk_name: none_if_empty(&bond.country_of_risk_name),
        sector: none_if_empty(&bond.sector),
        exchange: none_if_empty(&bond.exchange),
        coupon_type,
        coupon_amount,
        next_coupon_date,
        floating_coupon: bond.floating_coupon_flag,
        amortization: bond.amortization_flag,
        perpetual: bond.perpetual_flag,
        buy_available: bond.buy_available_flag,
        sell_available: bond.sell_available_flag,
    }))
}
