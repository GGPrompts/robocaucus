#!/usr/bin/env bash
set -e

BACKEND_PORT=17431
FRONTEND_PORT=17430
ROOT="$(cd "$(dirname "$0")" && pwd)"

if [[ ! -x "$ROOT/frontend/node_modules/.bin/vite" ]]; then
    echo "Frontend dependencies are missing. Run:"
    echo "  npm --prefix \"$ROOT/frontend\" ci"
    exit 1
fi

check_port() {
    ss -tlnp 2>/dev/null | grep -q ":$1 "
}

if check_port "$BACKEND_PORT"; then
    echo "Backend already running on port $BACKEND_PORT, skipping."
else
    echo "Starting backend..."
    cd "$ROOT"
    PORT="$BACKEND_PORT" cargo run -p backend &
    BACKEND_PID=$!
    echo "Backend PID: $BACKEND_PID"
fi

if check_port "$FRONTEND_PORT"; then
    echo "Frontend already running on port $FRONTEND_PORT, skipping."
else
    echo "Starting frontend..."
    cd "$ROOT/frontend"
    npm run dev &
    FRONTEND_PID=$!
    echo "Frontend PID: $FRONTEND_PID"
fi

echo ""
echo "Backend:  http://localhost:$BACKEND_PORT"
echo "Frontend: http://localhost:$FRONTEND_PORT"
echo ""
echo "Press Ctrl+C to stop."

trap 'echo "Stopping..."; kill $(jobs -p) 2>/dev/null; exit 0' INT TERM
wait
