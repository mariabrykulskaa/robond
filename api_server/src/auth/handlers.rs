use axum::extract::State;
use axum::Json;

use crate::auth::jwt::{decode_token, encode_access_token, encode_refresh_token};
use crate::auth::models::*;
use crate::auth::password::{hash_password, verify_password};
use crate::error::AppError;
use crate::state::AppState;

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::BadRequest("email and password are required".into()));
    }
    if req.password.len() < 6 {
        return Err(AppError::BadRequest("password must be at least 6 characters".into()));
    }

    let password_hash = hash_password(&req.password).map_err(|e| AppError::Internal(e.to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO app_user (email, password_hash) VALUES ($1, $2) RETURNING id, email, password_hash, created_at",
    )
    .bind(&req.email)
    .bind(&password_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("app_user_email_key") => {
            AppError::Conflict("email already registered".into())
        }
        other => AppError::Internal(other.to_string()),
    })?;

    let access_token =
        encode_access_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;
    let refresh_token =
        encode_refresh_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            email: user.email,
        },
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT id, email, password_hash, created_at FROM app_user WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::Unauthorized)?;

    let valid = verify_password(&req.password, &user.password_hash).map_err(|e| AppError::Internal(e.to_string()))?;
    if !valid {
        return Err(AppError::Unauthorized);
    }

    let access_token =
        encode_access_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;
    let refresh_token =
        encode_refresh_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            email: user.email,
        },
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = decode_token(&req.refresh_token, &state.jwt_secret)?;

    let user = sqlx::query_as::<_, User>("SELECT id, email, password_hash, created_at FROM app_user WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::Unauthorized)?;

    let access_token =
        encode_access_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;
    let refresh_token =
        encode_refresh_token(user.id, &state.jwt_secret).map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: UserInfo {
            id: user.id,
            email: user.email,
        },
    }))
}
