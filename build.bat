@echo off
setlocal
set ROOT=%~dp0
set FRONTEND=%ROOT%frontend

echo =========================================
echo  Tunny Dashboard - Full Production Build
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

echo [2/2] Building frontend (TypeScript + Vite)...
cd /d "%FRONTEND%"
call npm run build
if errorlevel 1 (
    echo.
    echo ERROR: Frontend build failed.
    pause
    exit /b 1
)

echo.
echo =========================================
echo  Build complete: frontend\dist\index.html
echo =========================================
pause
