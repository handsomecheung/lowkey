use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use sha2::{Digest, Sha256};

fn get_key_bytes(key: &str) -> [u8; 32] {
    // Hash the key using SHA256 to get a fixed 32-byte key
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&result);
    key_array
}

/// Encrypts plaintext using ChaCha20-Poly1305
/// Returns: nonce (12 bytes) + ciphertext + tag (16 bytes)
///
/// # Arguments
/// * `plaintext` - The data to encrypt
/// * `key` - Encryption key.
pub fn encrypt(plaintext: &[u8], key: &str) -> Result<Vec<u8>, String> {
    let key_bytes = get_key_bytes(key);
    let cipher = ChaCha20Poly1305::new((&key_bytes).into());
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
///
/// # Arguments
/// * `encrypted_data` - The data to decrypt (nonce + ciphertext + tag)
/// * `key` - Decryption key.
pub fn decrypt(encrypted_data: &[u8], key: &str) -> Result<Vec<u8>, String> {
    if encrypted_data.len() < 12 + 16 {
        return Err(format!(
            "Encrypted data too short: {} bytes (minimum is 28 bytes)",
            encrypted_data.len()
        ));
    }

    let key_bytes = get_key_bytes(key);
    let cipher = ChaCha20Poly1305::new((&key_bytes).into());
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
        let encrypted = encrypt(message, "default-key").unwrap();
        let decrypted = decrypt(&encrypted, "default-key").unwrap();
        assert_eq!(message, &decrypted[..]);
    }

    #[test]
    fn test_encrypt_decrypt_with_custom_key() {
        let message = b"Hello, World!";
        let custom_key = "my-custom-password";
        let encrypted = encrypt(message, custom_key).unwrap();
        let decrypted = decrypt(&encrypted, custom_key).unwrap();
        assert_eq!(message, &decrypted[..]);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let message = b"Hello, World!";
        let key1 = "correct-password";
        let key2 = "wrong-password";
        let encrypted = encrypt(message, key1).unwrap();
        let result = decrypt(&encrypted, key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let result = decrypt(&[0u8; 10], "any-key");
        assert!(result.is_err());
    }
}
