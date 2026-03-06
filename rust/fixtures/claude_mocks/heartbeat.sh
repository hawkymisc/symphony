#!/bin/bash
# Mock claude CLI that emits a heartbeat every 50ms for testing cancellation.
# This allows the runner to receive output immediately, so cancellation
# can be detected without waiting for the read timeout.
while true; do
    printf '{"type":"assistant","message":{"content":"Working..."}}\n'
    sleep 0.05
done
