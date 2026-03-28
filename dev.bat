@echo off
setlocal
set ROOT=%~dp0
set FRONTEND=%ROOT%frontend

echo =========================================
echo  Tunny Dashboard - Development Server
echo =========================================
echo.
echo Starting Vite dev server at http://localhost:5173
echo Press Ctrl+C to stop.
echo.

cd /d "%FRONTEND%"
call npm run dev
