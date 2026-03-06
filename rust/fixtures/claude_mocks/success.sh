#!/bin/bash
# Mock claude CLI that emits a successful stream-json sequence.
# Ignores all arguments.
printf '{"type":"assistant","message":{"content":"Working on it..."}}\n'
printf '{"type":"tool_use","tool":"Bash","input":{"command":"echo hello"}}\n'
printf '{"type":"tool_result","output":"hello"}\n'
printf '{"type":"result","result":"Task completed successfully.","usage":{"input_tokens":150,"output_tokens":50}}\n'
exit 0
