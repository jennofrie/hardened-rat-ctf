//! Session management for the C2 server.
//!
//! Tracks connected implants, supports multiple simultaneous sessions.

use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

/// A single implant session.
#[derive(Clone)]
pub struct Session {
    pub stream: Arc<Mutex<TcpStream>>,
    pub peer: String,
    pub connected_at: String,
}

impl Session {
    pub fn new(stream: TcpStream, peer: String) -> Self {
        let connected_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        Self {
            stream: Arc::new(Mutex::new(stream)),
            peer,
            connected_at,
        }
    }
}

/// Manages all active sessions.
pub struct SessionManager {
    sessions: HashMap<usize, Session>,
    next_id: usize,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add a new session and return its ID.
    pub fn add(&mut self, session: Session) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.sessions.insert(id, session);
        id
    }

    /// Get a session by ID.
    pub fn get(&self, id: usize) -> Option<&Session> {
        self.sessions.get(&id)
    }

    /// Remove a session by ID.
    pub fn remove(&mut self, id: usize) {
        if let Some(session) = self.sessions.remove(&id) {
            if let Ok(stream) = session.stream.lock() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        }
    }

    /// Print all active sessions.
    pub fn list(&self) {
        if self.sessions.is_empty() {
            println!("  No active sessions.");
            return;
        }
        println!("\n  {:<6} {:<24} {:<20}", "ID", "Peer", "Connected At");
        println!("  {}", "-".repeat(52));
        for (id, session) in &self.sessions {
            println!(
                "  {:<6} {:<24} {:<20}",
                id, session.peer, session.connected_at
            );
        }
        println!();
    }
}
