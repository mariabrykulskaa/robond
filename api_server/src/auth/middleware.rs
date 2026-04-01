use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::auth::jwt::decode_token;
use crate::state::AppState;

/// Extractor that validates the Bearer token and provides the authenticated user_id.
pub struct AuthUser(pub i64);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "unauthorized" }))).into_response()
    }
}

pub struct AuthError;

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError)?;

        let token = header.strip_prefix("Bearer ").ok_or(AuthError)?;
        let claims = decode_token(token, &state.jwt_secret).map_err(|_| AuthError)?;
        Ok(AuthUser(claims.sub))
    }
}
