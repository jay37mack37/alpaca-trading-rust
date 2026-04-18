@echo off
setlocal enabledelayedexpansion

echo =============================================
echo  AutoStonks Algo Suite - Overhaul Launcher
echo =============================================

REM ── Kill any prior instance running on port 8080 ──
for /f "tokens=5" %%p in ('netstat -aon ^| findstr ":8080 " ^| findstr LISTENING') do (
    echo Killing existing backend on port 8080 (PID %%p)...
    taskkill /PID %%p /F >nul 2>&1
)

REM ── Kill any prior Vite dev server ──
for /f "tokens=5" %%p in ('netstat -aon ^| findstr ":5173 " ^| findstr LISTENING') do (
    echo Killing existing frontend on port 5173 (PID %%p)...
    taskkill /PID %%p /F >nul 2>&1
)

timeout /t 1 /nobreak >nul

REM ── Load Alpaca keys from .env ──
if exist ".env" (
    for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
        set "line=%%a"
        if not "!line:~0,1!"=="#" (
            set "%%a=%%b"
        )
    )
)

REM ── Create data directory if needed ──
if not exist "data" mkdir data

REM ── Start backend ──
echo.
echo [1/2] Starting backend (port 8080)...
set AUTO_STONKS_HOST=127.0.0.1
set AUTO_STONKS_PORT=8080
set AUTO_STONKS_ALLOWED_ORIGINS=http://127.0.0.1:5173,http://localhost:5173
start "AutoStonks Backend" cmd /c "cargo run 2>&1 | tee data\backend.log"

REM ── Wait for backend to be ready ──
echo Waiting for backend...
:wait_backend
timeout /t 2 /nobreak >nul
curl -s http://127.0.0.1:8080/api/health >nul 2>&1
if errorlevel 1 goto wait_backend
echo Backend is ready!

REM ── Check if ui/.env exists, prompt if VITE_API_TOKEN is missing ──
if not exist "ui\.env" (
    echo.
    echo [!] ui\.env not found. Creating from example...
    echo     Check the backend log above for your API token, then update ui\.env
    copy "ui\.env.example" "ui\.env" >nul
)

REM ── Start frontend ──
echo.
echo [2/2] Starting frontend (port 5173)...
start "AutoStonks Frontend" cmd /c "cd ui && npm run dev"

echo.
echo =============================================
echo  Both services are running:
echo    Backend  : http://127.0.0.1:8080
echo    Frontend : http://127.0.0.1:5173
echo =============================================
echo.
echo NOTE: If the frontend shows a token error, check the
echo backend log window for the API token and add it to ui\.env
echo Then restart the frontend (Ctrl+C in its window, re-run start.bat).
echo.
pause
