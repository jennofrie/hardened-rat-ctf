//! Length-prefixed framing for encrypted C2 communication.
//!
//! Every message on the wire is:
//! ```text
//! [4-byte big-endian length] [encrypted payload]
//! ```
//!
//! The encryption/decryption is delegated to [`HybridCipher`].

use crate::crypto::HybridCipher;
use std::io::{self, Read, Write};

/// Maximum single-message size (1 MiB) to prevent memory exhaustion.
pub const MAX_MESSAGE_SIZE: u32 = 1024 * 1024;

/// Framed writer — sends length-prefixed, encrypted messages.
pub struct FramedWriter<W: Write> {
    inner: W,
    cipher: HybridCipher,
}

impl<W: Write> FramedWriter<W> {
    pub fn new(inner: W, cipher: HybridCipher) -> Self {
        Self { inner, cipher }
    }

    /// Encrypt and send `data` with a 4-byte length prefix.
    pub fn send(&mut self, data: &[u8]) -> io::Result<()> {
        let encrypted = self.cipher.encrypt(data);
        let len = encrypted.len() as u32;
        self.inner.write_all(&len.to_be_bytes())?;
        self.inner.write_all(&encrypted)?;
        self.inner.flush()?;
        Ok(())
    }
}

/// Framed reader — receives length-prefixed, encrypted messages.
pub struct FramedReader<R: Read> {
    inner: R,
    cipher: HybridCipher,
}

impl<R: Read> FramedReader<R> {
    pub fn new(inner: R, cipher: HybridCipher) -> Self {
        Self { inner, cipher }
    }

    /// Receive and decrypt a single framed message.
    /// Returns `None` on EOF or decryption failure.
    pub fn recv(&mut self) -> io::Result<Option<Vec<u8>>> {
        // Read 4-byte length
        let mut len_buf = [0u8; 4];
        match self.inner.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let len = u32::from_be_bytes(len_buf);
        if len > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("message too large: {len} bytes"),
            ));
        }

        // Read encrypted payload
        let mut buf = vec![0u8; len as usize];
        self.inner.read_exact(&mut buf)?;

        // Decrypt
        match self.cipher.decrypt(&buf) {
            Some(plaintext) => Ok(Some(plaintext)),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "decryption failed",
            )),
        }
    }

    /// Receive and decrypt, returning the result as a UTF-8 string.
    pub fn recv_string(&mut self) -> io::Result<Option<String>> {
        match self.recv()? {
            Some(data) => {
                let s = String::from_utf8_lossy(&data).into_owned();
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn test_cipher() -> HybridCipher {
        HybridCipher::new(b"TestRC4Key", b"TestAES128Key!!!")
    }

    #[test]
    fn framed_roundtrip() {
        let msg = b"Hello from C2";

        // Write
        let mut buf = Vec::new();
        {
            let mut writer = FramedWriter::new(&mut buf, test_cipher());
            writer.send(msg).unwrap();
        }

        // Read
        let mut reader = FramedReader::new(Cursor::new(&buf), test_cipher());
        let received = reader.recv().unwrap().unwrap();
        assert_eq!(received, msg);
    }

    #[test]
    fn framed_multiple_messages() {
        let messages = vec![b"first".to_vec(), b"second".to_vec(), b"third".to_vec()];

        let mut buf = Vec::new();
        {
            let mut writer = FramedWriter::new(&mut buf, test_cipher());
            for m in &messages {
                writer.send(m).unwrap();
            }
        }

        let mut reader = FramedReader::new(Cursor::new(&buf), test_cipher());
        for m in &messages {
            let received = reader.recv().unwrap().unwrap();
            assert_eq!(&received, m);
        }
        // EOF
        assert!(reader.recv().unwrap().is_none());
    }
}
