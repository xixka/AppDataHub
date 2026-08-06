//! 简单的混淆加密模块
//!
//! 使用 SHA-256 + XOR 实现轻量混淆

use base64::{engine::general_purpose, Engine};
use sha2::{Digest, Sha256};

fn derive_key(password: &str, salt: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn xor_bytes(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

pub fn encrypt(plaintext: &str, password: &str, salt: &str) -> String {
    let key = derive_key(password, salt);
    let encrypted = xor_bytes(plaintext.as_bytes(), &key);
    general_purpose::STANDARD.encode(&encrypted)
}

pub fn decrypt(ciphertext: &str, password: &str, salt: &str) -> Result<String, String> {
    let key = derive_key(password, salt);
    let decoded = general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(|e| format!("Base64 解码失败: {}", e))?;
    let decrypted = xor_bytes(&decoded, &key);
    String::from_utf8(decrypted).map_err(|e| format!("UTF-8 解码失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = "hello world 你好世界";
        let encrypted = encrypt(plaintext, "pw", "salt");
        assert_ne!(encrypted, plaintext);
        let decrypted = decrypt(&encrypted, "pw", "salt").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_password() {
        let plaintext = "secret data";
        let encrypted = encrypt(plaintext, "correct_pw", "salt");
        let result = decrypt(&encrypted, "wrong_pw", "salt");
        match result {
            Ok(decrypted) => assert_ne!(decrypted, plaintext),
            Err(_) => {}
        }
    }
}
