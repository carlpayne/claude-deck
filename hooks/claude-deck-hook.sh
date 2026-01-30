#!/bin/bash
# Claude Code hook for claude-deck integration
# This script receives hook events from Claude Code and updates the status file

STATUS_FILE="/tmp/claude-deck-status.json"
DEBUG_LOG="/tmp/claude-deck-hook-debug.log"

# Read JSON input from stdin
INPUT=$(cat)

# Debug: log raw input (comment out for production)
# echo "$(date): $INPUT" >> "$DEBUG_LOG"

# Extract event type and data using jq (if available) or basic parsing
if command -v jq &> /dev/null; then
    # Claude Code uses hook_event_name, not event
    EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // empty')
    TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
    TOOL_INPUT=$(echo "$INPUT" | jq -r '.tool_input // empty')
    MESSAGE=$(echo "$INPUT" | jq -r '.message // empty')
    # Note: Claude Code hooks don't include session data (cost/tokens/model)
    # We preserve cost/tokens from status file, but read model from settings
    if [ -f "$STATUS_FILE" ]; then
        SESSION_COST=$(jq -r '.cost // 0' "$STATUS_FILE")
        SESSION_TOKENS=$(jq -r '.tokens // 0' "$STATUS_FILE")
    else
        SESSION_COST="0"
        SESSION_TOKENS="0"
    fi
    # Read model from Claude Code settings
    SETTINGS_FILE="$HOME/.claude/settings.json"
    if [ -f "$SETTINGS_FILE" ]; then
        MODEL=$(jq -r '.model // empty' "$SETTINGS_FILE")
    else
        MODEL=""
    fi
else
    # Fallback: basic grep parsing
    EVENT=$(echo "$INPUT" | grep -o '"hook_event_name":"[^"]*"' | cut -d'"' -f4)
    TOOL_NAME=$(echo "$INPUT" | grep -o '"tool_name":"[^"]*"' | cut -d'"' -f4)
    MESSAGE=$(echo "$INPUT" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
    SESSION_COST="0"
    SESSION_TOKENS="0"
    MODEL=""
fi

TIMESTAMP=$(date +%s)

# Determine task and state based on event type
case "$EVENT" in
    "PreToolUse")
        TASK="$TOOL_NAME"
        PROCESSING="true"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    "PostToolUse")
        TASK="$TOOL_NAME done"
        PROCESSING="false"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    "Notification")
        # Check if it's a permission request
        if echo "$MESSAGE" | grep -qi "permission\|approve\|allow\|confirm"; then
            TASK="Waiting for permission"
            PROCESSING="false"
            WAITING="true"
            INPUT_TYPE='"permission"'
        elif echo "$MESSAGE" | grep -qi "error\|failed"; then
            TASK="Error"
            PROCESSING="false"
            WAITING="false"
            INPUT_TYPE="null"
        else
            TASK="$MESSAGE"
            PROCESSING="true"
            WAITING="false"
            INPUT_TYPE="null"
        fi
        ;;
    "Stop")
        TASK="Stopped"
        PROCESSING="false"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    *)
        # Unknown event, just update timestamp
        if [ -f "$STATUS_FILE" ]; then
            # Update timestamp only
            if command -v jq &> /dev/null; then
                jq ".timestamp = $TIMESTAMP" "$STATUS_FILE" > "${STATUS_FILE}.tmp" && mv "${STATUS_FILE}.tmp" "$STATUS_FILE"
            fi
        fi
        exit 0
        ;;
esac

# Handle empty model
if [ -z "$MODEL" ] || [ "$MODEL" = "null" ]; then
    MODEL_JSON="null"
else
    MODEL_JSON="\"$MODEL\""
fi

# Truncate task if too long
if [ ${#TASK} -gt 20 ]; then
    TASK="${TASK:0:17}..."
fi

# Write status file
cat > "$STATUS_FILE" << EOF
{
  "task": "$TASK",
  "waiting_for_input": $WAITING,
  "input_type": $INPUT_TYPE,
  "model": $MODEL_JSON,
  "cost": $SESSION_COST,
  "tokens": $SESSION_TOKENS,
  "processing": $PROCESSING,
  "error": null,
  "timestamp": $TIMESTAMP
}
EOF

exit 0
