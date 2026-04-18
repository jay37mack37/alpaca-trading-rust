@echo off
setlocal enabledelayedexpansion

echo =============================================
echo  AutoStonks Algo Suite - Overhaul Launcher
echo =============================================

echo Cleaning up port 8080...
powershell -Command "Get-NetTCPConnection -LocalPort 8080 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo Cleaning up port 5173...
powershell -Command "Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"



timeout /t 1 /nobreak >nul

echo Loading environment from .env...
if not exist ".env" goto no_env
for /f "usebackq tokens=1,* delims==" %%a in (".env") do (
    set "key=%%a"
    set "val=%%b"
    if defined key (
        if not "!key:~0,1!"=="#" (
            set "!key!=!val!"
        )
    )
)
:no_env

if not exist "data" mkdir data

echo.
echo [1/2] Starting backend (port 8080)...
set AUTO_STONKS_HOST=127.0.0.1
set AUTO_STONKS_PORT=8080
set AUTO_STONKS_ALLOWED_ORIGINS=http://127.0.0.1:5173,http://localhost:5173

start "AutoStonks Backend" powershell -NoExit -Command "cargo run 2>&1 | Tee-Object -FilePath data\backend.log"

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
echo [ERROR] Backend failed to start.
pause
exit /b 1

:backend_ready
echo Backend is ready!

if exist "ui\.env" goto frontend_env_ok
copy "ui\.env.example" "ui\.env" >nul
:frontend_env_ok

echo [2/2] Starting frontend (port 5173)...
if exist "ui\node_modules" goto npm_ok
echo [!] node_modules missing. Running npm install...
start "AutoStonks Frontend Install" cmd /c "cd ui && npm install"
pause
:npm_ok

start "AutoStonks Frontend" cmd /c "cd ui && npm run dev"

echo.
echo =============================================
echo  Both services are running!
echo =============================================
pause
