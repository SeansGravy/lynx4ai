#!/bin/bash

# Define the path to the lynx-mcp binary
LYNX_BIN="./target/release/lynx-mcp"
GEMINI_TEMP_DIR="/Users/seansgravy/.gemini/tmp/0ad70affc30bbc3172cc569b2be4c4f595f60d73cd1f53d31bac350302e5a7d4"
STDERR_LOG="$GEMINI_TEMP_DIR/lynx-mcp_stderr.log"

# --- Test 1: instance_create ---
echo "--- Running instance_create ---"
INSTANCE_CREATE_REQ='{"jsonrpc": "2.0", "id": 1, "method": "instance_create", "params": {"profile": "test_profile", "headless": true}}'

echo "Sending request:"
echo "$INSTANCE_CREATE_REQ" | jq . # Pretty print the request

# Create a unique named pipe for stdout
FIFO_OUT="/tmp/lynx-mcp_fifo_out_$$"
mkfifo "$FIFO_OUT"

# Redirect lynx-mcp's stderr to a log file and its stdout to the named pipe
# Run in background to let the main script continue
# IMPORTANT: The <(...) syntax provides the input to the command.
# "$LYNX_BIN" is the executable
# 1>"FIFO_OUT" redirects stdout to our named pipe
# 2>"STDERR_LOG" redirects stderr to our log file
"$LYNX_BIN" <(echo "$INSTANCE_CREATE_REQ") 1>"$FIFO_OUT" 2>"$STDERR_LOG" &
LYNX_PID=$!

# Read from the named pipe, which contains the JSON-RPC response
INSTANCE_CREATE_RESPONSE=$(cat "$FIFO_OUT")

# Clean up
rm "$FIFO_OUT"

# Wait for lynx-mcp to exit and kill it just in case
wait "$LYNX_PID" 2>/dev/null
kill "$LYNX_PID" 2>/dev/null

echo "Received raw response:"
echo "$INSTANCE_CREATE_RESPONSE"

echo "Content of $STDERR_LOG:"
cat "$STDERR_LOG"

# Try to extract instance ID (assuming success and 'result' field)
INSTANCE_ID=$(echo "$INSTANCE_CREATE_RESPONSE" | jq -r '.result | capture("Instance created: (?<id>[a-f0-9-]+)").id' 2>/dev/null)

if [ -z "$INSTANCE_ID" ]; then
    echo "ERROR: Failed to extract instance ID from response. Raw response was:"
    echo "$INSTANCE_CREATE_RESPONSE"
    echo "And stderr log was:"
    cat "$STDERR_LOG"
    exit 1
fi

echo "Extracted Instance ID: $INSTANCE_ID"

# The rest of the script (navigate, snapshot, destroy) will also need to be run
# with the same pattern of capturing response and stderr.
# For debugging the first step, we'll stop here.
# --- Test 2: navigate ---
# echo "--- Running navigate ---"
# ...

# --- Test 3: snapshot ---
# echo "--- Running snapshot ---"
# ...

# --- Test 4: instance_destroy ---
# echo "--- Running instance_destroy ---"
# ...

# echo "Basic functional tests completed successfully."
