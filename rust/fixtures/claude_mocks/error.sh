#!/bin/bash
# Mock claude CLI that emits an error event.
printf '{"type":"error","error":{"message":"Rate limit exceeded"}}\n'
exit 0
