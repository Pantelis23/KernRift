use std::fs;
use std::path::Path;

use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

fn nibble_to_hex(n: u8) -> char {
    debug_assert!(n < 16);
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => unreachable!(),
    }
}

pub(super) fn normalize_hex(
    text: &str,
    expected_len: usize,
    label: &str,
) -> Result<String, String> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.len() != expected_len {
        return Err(format!(
            "invalid hex in '{}': expected {} hex chars, got {}",
            label,
            expected_len,
            normalized.len()
        ));
    }
    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "invalid hex in '{}': value contains non-hex characters",
            label
        ));
    }
    Ok(normalized)
}

fn decode_hex_fixed<const N: usize>(text: &str, label: &str) -> Result<[u8; N], String> {
    let normalized = normalize_hex(text, N * 2, label)?;
    let mut out = [0_u8; N];
    let bytes = normalized.as_bytes();
    for i in 0..N {
        let hi = hex_char_to_nibble(bytes[i * 2] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        let lo = hex_char_to_nibble(bytes[i * 2 + 1] as char)
            .ok_or_else(|| format!("invalid hex in '{}': bad nibble", label))?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_char_to_nibble(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some((c as u8) - b'0'),
        'a'..='f' => Some((c as u8) - b'a' + 10),
        'A'..='F' => Some((c as u8) - b'A' + 10),
        _ => None,
    }
}

pub(super) fn load_signing_key_hex(path: &str) -> Result<SigningKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read signing key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

pub(super) fn load_verifying_key_hex(path: &str) -> Result<VerifyingKey, String> {
    let text = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("failed to read public key '{}': {}", path, e))?;
    let key_bytes = decode_hex_fixed::<32>(&text, path)?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid public key '{}': {}", path, e))
}
