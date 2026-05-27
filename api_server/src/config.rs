use std::env;

use base64::{engine::general_purpose::STANDARD as B64, Engine};

pub struct ApiConfig {
    pub jwt_secret: String,
    pub listen_addr: String,
    pub token_encryption_key: [u8; 32],
}

impl ApiConfig {
    pub fn from_env() -> Self {
        let listen_addr = if let Ok(port) = env::var("PORT") {
            format!("0.0.0.0:{port}")
        } else {
            env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        };

        let key_b64 =
            env::var("TOKEN_ENCRYPTION_KEY").expect("TOKEN_ENCRYPTION_KEY must be set (base64-encoded 32-byte key)");
        let key_bytes = B64.decode(&key_b64).expect("TOKEN_ENCRYPTION_KEY is not valid base64");
        let token_encryption_key: [u8; 32] = key_bytes
            .try_into()
            .expect("TOKEN_ENCRYPTION_KEY must be exactly 32 bytes");

        Self {
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            listen_addr,
            token_encryption_key,
        }
    }
}
