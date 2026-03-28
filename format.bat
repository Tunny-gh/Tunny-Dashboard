@echo off
setlocal
set ROOT=%~dp0
set FRONTEND=%ROOT%frontend

echo =========================================
echo  Tunny Dashboard - Format
echo =========================================
echo.

echo [1/2] Formatting Rust (cargo fmt)...
cd /d "%ROOT%rust_core"
cargo fmt
if errorlevel 1 (
    echo.
    echo ERROR: cargo fmt failed.
    pause
    exit /b 1
)
echo Done.
echo.

echo [2/2] Formatting TypeScript/TSX (Prettier)...
cd /d "%FRONTEND%"
call npx --yes prettier --write "src/**/*.{ts,tsx}"
if errorlevel 1 (
    echo.
    echo ERROR: Prettier failed.
    pause
    exit /b 1
)
echo Done.
echo.

echo =========================================
echo  Formatting complete.
echo =========================================
pause
