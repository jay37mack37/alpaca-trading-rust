#!/usr/bin/env bash
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "============================================="
echo " AutoStonks Algo Suite - Stop"
echo "============================================="

stopped=0

for port in 8080 3000 5173; do
    pids=$(lsof -ti:$port 2>/dev/null || true)
    if [ -n "$pids" ]; then
        echo "Killing processes on port $port: $pids"
        kill $pids 2>/dev/null || true
        stopped=1
    else
        echo "No WSL process found on port $port"
    fi

    # Also kill Windows-side processes (handles shells spawned by start.bat)
    win_pids=$(powershell.exe -Command "Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess" 2>/dev/null || true)
    if [ -n "$win_pids" ]; then
        echo "Killing Windows processes on port $port: $win_pids"
        echo "$win_pids" | while read -r pid; do
            [ -n "$pid" ] && powershell.exe -Command "Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue" 2>/dev/null || true
        done
        stopped=1
    fi
done

# Kill any cargo run child processes
for pid in $(pgrep -f "cargo run" 2>/dev/null || true); do
    echo "Killing cargo process: $pid"
    kill $pid 2>/dev/null || true
    stopped=1
done

# Kill Windows-side cargo processes
powershell.exe -Command "Get-Process -Name 'autostonks-backend','cargo' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue" 2>/dev/null || true

if [ "$stopped" -eq 1 ]; then
    echo ""
    echo "Waiting for processes to exit..."
    sleep 2
    echo "All services stopped."
else
    echo ""
    echo "No services were running."
fi