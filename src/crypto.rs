use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};

const HARD_CODED_KEY: &[u8; 32] = b"my-secret-key-32-bytes-long!!!!!";

/// Encrypts plaintext using ChaCha20-Poly1305
/// Returns: nonce (12 bytes) + ciphertext + tag (16 bytes)
pub fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(HARD_CODED_KEY.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypts ciphertext using ChaCha20-Poly1305
/// Input format: nonce (12 bytes) + ciphertext + tag (16 bytes)
pub fn decrypt(encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
    if encrypted_data.len() < 12 + 16 {
        return Err(format!(
            "Encrypted data too short: {} bytes (minimum is 28 bytes)",
            encrypted_data.len()
        ));
    }

    let cipher = ChaCha20Poly1305::new(HARD_CODED_KEY.into());
    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let message = b"Hello, World!";
        let encrypted = encrypt(message).unwrap();
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(message, &decrypted[..]);
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let result = decrypt(&[0u8; 10]);
        assert!(result.is_err());
    }
}
