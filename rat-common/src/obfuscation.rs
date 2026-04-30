//! Compile-time and runtime string obfuscation utilities.
//!
//! Provides XOR-based obfuscation that mirrors the original C implementation
//! and a `obfuscate!` macro for compile-time string encryption.

/// Default XOR key matching the original C implant (0xAB).
pub const XOR_KEY: u8 = 0xAB;

/// XOR-encrypt `data` in place with the given `key` byte.
pub fn xor_encrypt(data: &mut [u8], key: u8) {
    for byte in data.iter_mut() {
        *byte ^= key;
    }
}

/// XOR-decrypt `data` and return a new `Vec<u8>`.
pub fn xor_decrypt(data: &[u8], key: u8) -> Vec<u8> {
    data.iter().map(|b| b ^ key).collect()
}

/// Decrypt an XOR-obfuscated byte slice into a UTF-8 string.
/// Returns `None` if the result is not valid UTF-8.
pub fn xor_decrypt_str(data: &[u8], key: u8) -> Option<String> {
    String::from_utf8(xor_decrypt(data, key)).ok()
}

// ---------------------------------------------------------------------------
// Compile-time string obfuscation macro
// ---------------------------------------------------------------------------

/// Obfuscate a string literal at compile time using XOR with `XOR_KEY`.
///
/// At runtime the macro produces a `String` by decrypting the embedded
/// byte array. This keeps cleartext strings out of the binary.
///
/// # Example
/// ```ignore
/// let s = obfuscate!("secret");
/// assert_eq!(s, "secret");
/// ```
#[macro_export]
macro_rules! obfuscate {
    ($s:expr) => {{
        // The const block evaluates at compile time, producing the
        // XOR'd byte array that gets baked into the binary.
        const INPUT: &[u8] = $s.as_bytes();
        const LEN: usize = INPUT.len();
        const fn xor_bytes() -> [u8; LEN] {
            let mut out = [0u8; LEN];
            let mut i = 0;
            while i < LEN {
                out[i] = INPUT[i] ^ $crate::obfuscation::XOR_KEY;
                i += 1;
            }
            out
        }
        const ENCRYPTED: [u8; LEN] = xor_bytes();
        // Runtime decryption
        $crate::obfuscation::xor_decrypt(&ENCRYPTED, $crate::obfuscation::XOR_KEY)
    }};
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_roundtrip() {
        let original = b"Hello, World!";
        let mut data = original.to_vec();
        xor_encrypt(&mut data, XOR_KEY);
        assert_ne!(&data, original);
        xor_encrypt(&mut data, XOR_KEY); // XOR is its own inverse
        assert_eq!(&data, original);
    }

    #[test]
    fn xor_decrypt_to_string() {
        let plain = "Test String";
        let enc: Vec<u8> = plain.bytes().map(|b| b ^ XOR_KEY).collect();
        assert_eq!(xor_decrypt_str(&enc, XOR_KEY).unwrap(), plain);
    }

    #[test]
    fn obfuscate_macro() {
        let decrypted = obfuscate!("MySecretKey2024!");
        assert_eq!(String::from_utf8(decrypted).unwrap(), "MySecretKey2024!");
    }
}
