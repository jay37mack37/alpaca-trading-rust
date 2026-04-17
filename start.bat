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
setlocal enabledelayedexpansion
set RETRY=0
:waitloop
powershell -Command "exit [int](!(Test-NetConnection -ComputerName 'localhost' -Port 3000).TcpTestSucceeded)" >nul 2>&1
if %ERRORLEVEL% neq 0 (
    set /A RETRY+=1
    if %RETRY% geq 20 (
        echo Server did not respond on port 3000 after 20 seconds.
        goto :startup_failed
    )
    timeout /t 1 /nobreak >nul
    goto waitloop
)

echo.
echo Starting frontend...
REM Open the frontend URL in the default web browser
start http://localhost:3000

echo Startup complete!
goto :eof

:startup_failed
echo Frontend startup aborted because the backend did not become ready.
exit /b 1
