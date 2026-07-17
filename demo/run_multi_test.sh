#!/usr/bin/env bash
set -e

# Build if needed (optional)
# cargo build --release --workspace

# Start the server in background
cargo run -p asp_demo_server &
SERVER_PID=$!

# Give server a moment to start
echo "Waiting for server to start..."
sleep 2

TEXT="Hello world"
TARGETS=("ru" "de" "fr" "es" "zh")

for t in "${TARGETS[@]}"; do
  echo "Translation to $t:"
  curl -s "http://127.0.0.1:8080/translate?text=${TEXT}&target=${t}" | jq .
  echo
done

# Stop the server
kill $SERVER_PID
