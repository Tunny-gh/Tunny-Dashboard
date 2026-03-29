@echo off
setlocal
set ROOT=%~dp0
set FRONTEND=%ROOT%frontend

echo =========================================
echo  Tunny Dashboard - Development Server
echo =========================================
echo.

echo [1/2] Building Rust/WASM core...
cd /d "%ROOT%rust_core"
wasm-pack build --target web --out-dir "%FRONTEND%\src\wasm\pkg"
if errorlevel 1 (
    echo.
    echo ERROR: WASM build failed.
    pause
    exit /b 1
)
echo Done.
echo.

echo [2/2] Starting Vite dev server at http://localhost:5173
echo Press Ctrl+C to stop.
echo.

cd /d "%FRONTEND%"
call npm run dev
