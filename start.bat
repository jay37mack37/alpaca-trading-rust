@echo off
echo Cleaning up previous running instances...
REM Force kill the previous running instance of the rust server
taskkill /IM alpaca_trading3web.exe /F /T >nul 2>&1

echo.
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo Checking whether Cargo is available...
where cargo >nul 2>&1
if errorlevel 1 (
    echo ERROR: Cargo was not found on your PATH.
    echo Install Rust using rustup: https://rustup.rs/
    echo Then reopen your terminal or add Cargo to PATH.
    echo.
    echo Press any key to exit...
pause >nul
    exit /b 1
)

echo.
echo Starting backend server...
REM Start the rust backend in a new command window so it stays running
start "Alpaca Trading Server" cmd /k "cargo run"

echo.
echo Waiting for server to start up...
timeout /t 3 /nobreak >nul

echo.
echo Starting frontend...
REM Open the frontend URL in the default web browser
start http://localhost:3000

echo Startup complete!
