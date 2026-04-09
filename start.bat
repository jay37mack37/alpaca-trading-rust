@echo off
echo Cleaning up previous running instances...
REM Force kill the previous running instance of the rust server
taskkill /IM alpaca_trading3web.exe /F /T >nul 2>&1

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
