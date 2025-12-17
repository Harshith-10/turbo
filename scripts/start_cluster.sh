#!/bin/bash

# Kill previous instances
pkill -f "turbo-leader"
pkill -f "turbo-worker"

# Build
cargo build

# Start Leader
echo "Starting Leader..."
./target/debug/turbo-leader &
LEADER_PID=$!
sleep 2

# Start Workers
echo "Starting 4 Workers..."
./target/debug/turbo-worker &
./target/debug/turbo-worker &
./target/debug/turbo-worker &
./target/debug/turbo-worker &

echo "Cluster started. Leader PID: $LEADER_PID"
echo "Press Ctrl+C to stop."

wait $LEADER_PID
