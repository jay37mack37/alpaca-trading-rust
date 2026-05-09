@echo off
setlocal enabledelayedexpansion

:: Ensure we run from the script's directory regardless of where it was launched from
cd /d "%~dp0"

echo =============================================
echo  AutoStonks Algo Suite - Overhaul Launcher
echo =============================================

:: --- Cleanup stale processes on our ports ---
echo Cleaning up port 8080...
powershell -Command "Get-NetTCPConnection -LocalPort 8080 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo Cleaning up port 5173...
powershell -Command "Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo Cleaning up port 3000...
powershell -Command "Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo Cleaning up port 3001...
powershell -Command "Get-NetTCPConnection -LocalPort 3001 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo Cleaning up port 8000 (Kronos)...
powershell -Command "Get-NetTCPConnection -LocalPort 8000 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

timeout /t 1 /nobreak >nul

:: --- AI Engine: Kronos Bridge ---
echo.
echo [0/2] Launching Kronos AI Brain...

:: Try to find the right python command
set "PYTHON_CMD=py -3.10"
py -3.10 --version >nul 2>&1
if %errorlevel% neq 0 (
    set "PYTHON_CMD=python"
)

:: Check for dependencies
%PYTHON_CMD% -c "import torch, transformers, fastapi, uvicorn, yfinance" 2>nul
if %errorlevel% neq 0 (
    echo [!] Missing AI dependencies. Installing...
    %PYTHON_CMD% -m pip install torch transformers fastapi uvicorn yfinance einops huggingface_hub tqdm safetensors
)

start "Kronos AI Bridge" cmd /k "cd backend && %PYTHON_CMD% kronos_bridge.py"
echo AI Brain is initializing in the background...
timeout /t 5 /nobreak >nul

:: --- Load .env if present ---
echo Loading environment from .env...
if not exist ".env" goto no_env
for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
    set "line_key=%%a"
    set "line_val=%%b"
    if defined line_key (
        set "first_char=!line_key:~0,1!"
        if not "!first_char!"=="#" (
            set "!line_key!=!line_val!"
        )
    )
)
:no_env

:: --- Validate required env vars ---
if "!AUTO_STONKS_MASTER_KEY!"=="" (
    echo [ERROR] AUTO_STONKS_MASTER_KEY is not set. Please check your .env file.
    pause
    exit /b 1
)
if "!AUTO_STONKS_API_TOKEN!"=="" (
    echo [INFO] AUTO_STONKS_API_TOKEN missing, backend will generate one.
)

if not exist "data" mkdir data

:: --- Backend ---
echo.
echo [1/2] Starting backend (port 8080)...
set AUTO_STONKS_HOST=127.0.0.1
set AUTO_STONKS_PORT=8080
set AUTO_STONKS_ALLOWED_ORIGINS=http://127.0.0.1:3000,http://localhost:3000,http://127.0.0.1:5173,http://localhost:5173,http://localhost:3001,http://127.0.0.1:3001

start "AutoStonks Backend" powershell -NoExit -Command "$env:AUTO_STONKS_HOST='127.0.0.1'; $env:AUTO_STONKS_PORT='8080'; $env:AUTO_STONKS_ALLOWED_ORIGINS='http://127.0.0.1:3000,http://localhost:3000,http://127.0.0.1:5173,http://localhost:5173,http://localhost:3001,http://127.0.0.1:3001'; $env:AUTO_STONKS_MASTER_KEY='!AUTO_STONKS_MASTER_KEY!'; $env:AUTO_STONKS_API_TOKEN='!AUTO_STONKS_API_TOKEN!'; cd backend; cargo run --bin autostonks-backend 2>&1 | Tee-Object -FilePath ..\data\backend.log"

echo Waiting for backend to initialize...
set "retries=0"
set "max_retries=60"

:wait_backend
set /a retries+=1
if %retries% geq %max_retries% goto backend_timeout

powershell -Command "try { $r = Invoke-WebRequest -Uri http://127.0.0.1:8080/api/health -UseBasicParsing -ErrorAction Stop; exit 0 } catch { exit 1 }" >nul 2>&1
if %errorlevel% equ 0 goto backend_ready

if %retries% equ 1 echo Still waiting...
timeout /t 2 /nobreak >nul
goto wait_backend

:backend_timeout
echo [ERROR] Backend failed to start. Check data\backend.log for details.
pause
exit /b 1

:backend_ready
echo Backend is ready!

:: --- Frontend ---
if exist "frontend\.env" goto frontend_env_ok
copy "frontend\.env.example" "frontend\.env" >nul
:frontend_env_ok

echo.
echo [2/2] Starting frontend (port 3000)...
if exist "frontend\node_modules" goto npm_ok
echo [!] node_modules missing. Running npm install...
start "AutoStonks Frontend Install" cmd /c "cd frontend && npm install"
pause
:npm_ok

start "AutoStonks Frontend" cmd /c "cd frontend && npm run dev"

echo.
echo =============================================
echo  Both services are running!
echo =============================================
echo   Backend:  http://127.0.0.1:8080
echo   Frontend: http://localhost:3000
echo =============================================
pause
