# CLAUDE.md — hardened-rat-ctf

## Project Overview

Advanced Windows Remote Access Tool (RAT) written in Rust for CTF competitions and security research. Rewritten from an original C + Python implementation to leverage Rust's safety, compile-time string obfuscation, and hybrid encryption.

## Architecture

Rust workspace with three crates:

- **rat-common** — Shared crypto (RC4, AES-128-CTR, HybridCipher), compile-time XOR string obfuscation (`obfuscate!` macro), and length-prefixed framed I/O protocol.
- **rat-implant** — Windows-targeting RAT binary. Uses conditional compilation (`#[cfg(target_os = "windows")]`) so `cargo check` passes on macOS/Linux. Modules: `config`, `dynapi`, `anti_analysis`, `keylogger`, `persistence`, `network`, `shell`.
- **rat-server** — Cross-platform C2 server with multi-session management and an interactive operator console.

## Build Commands

```bash
# Check entire workspace (works on macOS — implant stubs out Windows code)
cargo check --workspace

# Build C2 server (runs on any OS)
cargo build --release -p rat-server

# Build implant for Windows (cross-compile)
rustup target add x86_64-pc-windows-gnu
cargo build --release -p rat-implant --target x86_64-pc-windows-gnu

# Run tests (rat-common has unit tests for crypto + protocol)
cargo test -p rat-common
```

## Key Design Decisions

1. **Compile-time obfuscation** — The `obfuscate!()` macro in `rat-common` uses `const fn` to XOR strings at compile time. No cleartext strings appear in the binary.
2. **Dynamic API resolution** — `rat-implant/src/dynapi.rs` resolves Win32 APIs via `LoadLibraryA`/`GetProcAddress` at runtime to avoid static import table entries.
3. **Hybrid encryption** — Every C2 message is encrypted with AES-128-CTR (random nonce) then wrapped in RC4. This defeats both pattern matching and replay attacks.
4. **Conditional compilation** — All Windows-specific code is gated with `#[cfg(target_os = "windows")]`. The implant's `main()` prints an error on non-Windows and exits.
5. **No unsafe in common crate** — All unsafe code is isolated in the implant crate where Win32 FFI requires it.

## Configuration

Edit `rat-implant/src/config.rs` to change C2 IP, port, encryption keys, or beacon intervals. All sensitive strings use the `obfuscate!()` macro.

## Encryption Keys (defaults — change for CTF)

- RC4 key: `MySecretKey2024!`
- AES key: `AES128CTFKey2024`
- XOR key: `0xAB`

## Dependencies

- `aes` + `ctr` — AES-128-CTR encryption
- `rand` — Nonce generation, jitter
- `windows-sys` — Win32 FFI bindings (implant only, Windows only)
- `chrono` — Timestamp formatting (server only)
- `serde` + `serde_json` — Serialisation (reserved for future protocol extensions)

## Legal

CTF / educational / authorised research only. MIT licensed. Author: Jennofrie Daguil.
