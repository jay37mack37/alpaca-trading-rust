#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "============================================="
echo " AutoStonks Algo Suite - Linux Launcher"
echo "============================================="

# --- Cleanup stale processes on our ports ---
echo "Cleaning up port 8080..."
pids=$(lsof -ti:8080 2>/dev/null || true)
if [ -n "$pids" ]; then
    echo "  Killing: $pids"
    kill $pids 2>/dev/null || true
fi

echo "Cleaning up port 5173..."
pids=$(lsof -ti:5173 2>/dev/null || true)
if [ -n "$pids" ]; then
    echo "  Killing: $pids"
    kill $pids 2>/dev/null || true
fi

echo "Cleaning up port 3000..."
pids=$(lsof -ti:3000 2>/dev/null || true)
if [ -n "$pids" ]; then
    echo "  Killing: $pids"
    kill $pids 2>/dev/null || true
fi

sleep 1

# --- Load .env if present ---
if [ -f ".env" ]; then
    echo "Loading environment from .env..."
    set -a
    while IFS='=' read -r key val || [ -n "$key" ]; do
        [[ -z "$key" || "$key" =~ ^# ]] && continue
        export "$key=$val"
    done < .env
    set +a
fi

# --- Validate required env vars ---
if [ -z "${AUTO_STONKS_MASTER_KEY:-}" ]; then
    echo "[ERROR] AUTO_STONKS_MASTER_KEY is not set. Please check your .env file."
    exit 1
fi
if [ -z "${AUTO_STONKS_API_TOKEN:-}" ]; then
    echo "[ERROR] AUTO_STONKS_API_TOKEN is not set. Please check your .env file."
    exit 1
fi

mkdir -p data

# --- Backend ---
echo ""
echo "[1/2] Starting backend on 127.0.0.1:8080..."
export AUTO_STONKS_HOST=127.0.0.1
export AUTO_STONKS_PORT=8080
export AUTO_STONKS_ALLOWED_ORIGINS="http://127.0.0.1:3000,http://localhost:3000,http://127.0.0.1:5173,http://localhost:5173,http://localhost:3001,http://127.0.0.1:3001"

cd backend
cargo run 2>&1 | tee ../data/backend.log &
BACKEND_PID=$!
cd ..

echo "Waiting for backend to initialize..."
retries=0
max_retries=60

while [ $retries -lt $max_retries ]; do
    if curl -sf http://127.0.0.1:8080/api/health > /dev/null 2>&1; then
        echo "Backend is ready!"
        break
    fi
    retries=$((retries + 1))
    if [ $retries -eq 1 ]; then
        echo "Still waiting..."
    fi
    sleep 2
done

if [ $retries -ge $max_retries ]; then
    echo "[ERROR] Backend failed to start. Check data/backend.log for details."
    kill $BACKEND_PID 2>/dev/null || true
    exit 1
fi

# --- Frontend ---
if [ ! -f "frontend/.env" ]; then
    cp "frontend/.env.example" "frontend/.env"
fi

echo ""
echo "[2/2] Starting frontend on 0.0.0.0:5173..."
if [ ! -d "frontend/node_modules" ]; then
    echo "[!] node_modules missing. Running npm install..."
    (cd frontend && npm install)
fi

(cd frontend && npm run dev) &
FRONTEND_PID=$!

echo ""
echo "============================================="
echo " Both services are running!"
echo "   Backend:  http://127.0.0.1:8080"
echo "   Frontend: http://localhost:5173"
echo ""
echo " PIDs: backend=$BACKEND_PID frontend=$FRONTEND_PID"
echo " Press Ctrl+C to stop both."
echo "============================================="

# Wait for either process to exit
wait -n $BACKEND_PID $FRONTEND_PID 2>/dev/null || true

echo "A process exited. Shutting down..."
kill $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
wait $BACKEND_PID $FRONTEND_PID 2>/dev/null || true
