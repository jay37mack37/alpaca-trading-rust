@echo off
setlocal enabledelayedexpansion

:: Load .env
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

echo [RESTART] Stopping port 8080...
powershell -Command "Get-NetTCPConnection -LocalPort 8080 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

echo [RESTART] Starting backend...
set AUTO_STONKS_HOST=127.0.0.1
set AUTO_STONKS_PORT=8080
set AUTO_STONKS_ALLOWED_ORIGINS=http://127.0.0.1:3000,http://localhost:3000,http://127.0.0.1:5173,http://localhost:5173,http://localhost:3001,http://127.0.0.1:3001

cd backend
cargo run --bin autostonks-backend
