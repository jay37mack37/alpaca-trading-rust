@echo off
echo =============================================
echo  AutoStonks Algo Suite - Stop
echo =============================================

set FOUND=0

echo Stopping backend (port 8080)...
powershell -Command "$conns = Get-NetTCPConnection -LocalPort 8080 -ErrorAction SilentlyContinue; if ($conns) { $conns | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }; Write-Host '  Stopped.' } else { Write-Host '  Not running.' }"

echo Stopping frontend (port 5173)...
powershell -Command "$conns = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue; if ($conns) { $conns | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }; Write-Host '  Stopped.' } else { Write-Host '  Not running.' }"

echo Stopping frontend (port 3000)...
powershell -Command "$conns = Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue; if ($conns) { $conns | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }; Write-Host '  Stopped.' } else { Write-Host '  Not running.' }"

echo Stopping frontend (port 3001)...
powershell -Command "$conns = Get-NetTCPConnection -LocalPort 3001 -ErrorAction SilentlyContinue; if ($conns) { $conns | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }; Write-Host '  Stopped.' } else { Write-Host '  Not running.' }"

echo.
echo All services stopped.
pause