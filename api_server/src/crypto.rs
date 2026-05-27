use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

use crate::error::AppError;

/// Шифрует plaintext с помощью AES-256-GCM.
/// Возвращает строку `base64(nonce || ciphertext)`.
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, AppError> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Internal(format!("encryption failed: {e}")))?;

    let mut blob = nonce.to_vec();
    blob.extend_from_slice(&ciphertext);
    Ok(B64.encode(blob))
}

/// Дешифрует строку `base64(nonce || ciphertext)` обратно в plaintext.
pub fn decrypt(encoded: &str, key: &[u8; 32]) -> Result<String, AppError> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let blob = B64
        .decode(encoded)
        .map_err(|e| AppError::Internal(format!("base64 decode failed: {e}")))?;

    if blob.len() < 12 {
        return Err(AppError::Internal("invalid encrypted data".into()));
    }

    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| AppError::Internal(format!("decryption failed: {e}")))?;

    String::from_utf8(plaintext).map_err(|e| AppError::Internal(format!("invalid utf8 after decryption: {e}")))
}
