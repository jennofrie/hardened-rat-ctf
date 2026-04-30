//! Cryptographic primitives: RC4 stream cipher and AES-CTR hybrid encryption.
//!
//! The hybrid scheme uses RC4 for the outer transport layer (fast, low overhead)
//! and AES-128-CTR for the inner payload (strong confidentiality). A random
//! 16-byte nonce is prepended to each AES-encrypted payload.

use aes::cipher::{KeyIvInit, StreamCipher};
use rand::Rng;

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

// ---------------------------------------------------------------------------
// RC4 stream cipher
// ---------------------------------------------------------------------------

/// RC4 (ARC4) stream cipher implementation.
///
/// Used as the transport-layer cipher for C2 framing. Each direction
/// should use a fresh [`Rc4`] instance initialised with the shared key
/// so the keystream resets per message (matching the original C impl).
pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    /// Key-schedule algorithm (KSA).
    pub fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (idx, slot) in s.iter_mut().enumerate() {
            *slot = idx as u8;
        }
        let mut j: u8 = 0;
        for i in 0..256u16 {
            j = j
                .wrapping_add(s[i as usize])
                .wrapping_add(key[(i as usize) % key.len()]);
            s.swap(i as usize, j as usize);
        }
        Self { s, i: 0, j: 0 }
    }

    /// Pseudo-random generation algorithm (PRGA) — XORs `data` in place.
    pub fn apply(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[self.s[self.i as usize].wrapping_add(self.s[self.j as usize]) as usize];
            *byte ^= k;
        }
    }
}

// ---------------------------------------------------------------------------
// AES-128-CTR layer
// ---------------------------------------------------------------------------

/// Encrypt `plaintext` with AES-128-CTR using a random nonce.
/// Returns `nonce (16 bytes) || ciphertext`.
pub fn aes_encrypt(key: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let nonce: [u8; 16] = rand::thread_rng().gen();
    let mut buf = plaintext.to_vec();
    let mut cipher = Aes128Ctr::new(key.into(), &nonce.into());
    cipher.apply_keystream(&mut buf);

    let mut out = Vec::with_capacity(16 + buf.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&buf);
    out
}

/// Decrypt `nonce || ciphertext` produced by [`aes_encrypt`].
pub fn aes_decrypt(key: &[u8; 16], data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 16 {
        return None;
    }
    let (nonce, ct) = data.split_at(16);
    let nonce: [u8; 16] = nonce.try_into().ok()?;
    let mut buf = ct.to_vec();
    let mut cipher = Aes128Ctr::new(key.into(), &nonce.into());
    cipher.apply_keystream(&mut buf);
    Some(buf)
}

// ---------------------------------------------------------------------------
// Hybrid cipher (RC4 transport + AES payload)
// ---------------------------------------------------------------------------

/// Hybrid encryption: AES-128-CTR for payload confidentiality wrapped in
/// RC4 for the transport frame.
///
/// ```text
/// encrypt(plaintext):
///   inner = AES-CTR(aes_key, plaintext)   // nonce || ciphertext
///   outer = RC4(rc4_key, inner)
///   return outer
/// ```
pub struct HybridCipher {
    rc4_key: Vec<u8>,
    aes_key: [u8; 16],
}

impl HybridCipher {
    pub fn new(rc4_key: &[u8], aes_key: &[u8; 16]) -> Self {
        Self {
            rc4_key: rc4_key.to_vec(),
            aes_key: *aes_key,
        }
    }

    /// Encrypt `plaintext` with the hybrid scheme.
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        // Inner AES-CTR layer
        let inner = aes_encrypt(&self.aes_key, plaintext);
        // Outer RC4 layer
        let mut outer = inner;
        let mut rc4 = Rc4::new(&self.rc4_key);
        rc4.apply(&mut outer);
        outer
    }

    /// Decrypt data produced by [`HybridCipher::encrypt`].
    pub fn decrypt(&self, data: &[u8]) -> Option<Vec<u8>> {
        // Strip RC4 layer
        let mut buf = data.to_vec();
        let mut rc4 = Rc4::new(&self.rc4_key);
        rc4.apply(&mut buf);
        // Strip AES-CTR layer
        aes_decrypt(&self.aes_key, &buf)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_roundtrip() {
        let key = b"TestKey123";
        let original = b"Hello, World!";
        let mut data = original.to_vec();

        Rc4::new(key).apply(&mut data);
        // Should be different from original
        assert_ne!(&data, original);
        // Decrypt
        Rc4::new(key).apply(&mut data);
        assert_eq!(&data, original);
    }

    #[test]
    fn aes_roundtrip() {
        let key = b"0123456789abcdef";
        let msg = b"The quick brown fox jumps over the lazy dog";
        let ct = aes_encrypt(key, msg);
        let pt = aes_decrypt(key, &ct).unwrap();
        assert_eq!(pt, msg);
    }

    #[test]
    fn hybrid_roundtrip() {
        let cipher = HybridCipher::new(b"RC4SharedKey!", b"AES128SharedKey!");
        let msg = b"Sensitive C2 payload data";
        let ct = cipher.encrypt(msg);
        let pt = cipher.decrypt(&ct).unwrap();
        assert_eq!(pt, msg);
    }
}
