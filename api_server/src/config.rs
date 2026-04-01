use std::env;

pub struct ApiConfig {
    pub jwt_secret: String,
    pub listen_addr: String,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self {
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
        }
    }
}
