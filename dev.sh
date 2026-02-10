#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

# ── find two free ports ─────────────────────────────────────────────
find_free_port() {
  local port="${1:-3000}"
  while : ; do
    if ! (echo >/dev/tcp/localhost/"$port") 2>/dev/null; then
      echo "$port"
      return
    fi
    port=$((port + 1))
    if [ "$port" -gt 65535 ]; then
      echo "No free port found" >&2
      exit 1
    fi
  done
}

FRONTEND_PORT=$(find_free_port 4000)
BACKEND_PORT=$(find_free_port $((FRONTEND_PORT + 1)))

export FRONTEND_PORT BACKEND_PORT
export VK_ALLOWED_ORIGINS="http://localhost:${FRONTEND_PORT}"
export DISABLE_WORKTREE_CLEANUP=1
export RUST_LOG=debug

echo "Ports: frontend=$FRONTEND_PORT  backend=$BACKEND_PORT"

# ── build backend ───────────────────────────────────────────────────
echo "Building backend..."
cargo build --bin server 2>&1

# ── start backend (background, but logs visible) ────────────────────
echo "Starting backend on port $BACKEND_PORT..."
cargo run --bin server &
BACKEND_PID=$!

cleanup() {
  echo ""
  echo "Shutting down..."
  kill "$BACKEND_PID" 2>/dev/null || true
  wait "$BACKEND_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# ── wait for backend to be ready ────────────────────────────────────
echo "Waiting for backend..."
for _ in $(seq 1 60); do
  if (echo >/dev/tcp/localhost/"$BACKEND_PORT") 2>/dev/null; then
    echo "Backend ready."
    break
  fi
  sleep 1
done

# ── emit the url marker ────────────────────────────────────────────
echo "\$VK-URL\$http://localhost:${FRONTEND_PORT}\$VK-URL\$"

# ── start frontend (foreground) ─────────────────────────────────────
cd frontend
VITE_OPEN=false exec pnpm run dev -- --port "$FRONTEND_PORT"
