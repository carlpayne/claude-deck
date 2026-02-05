#!/bin/bash
# Claude Code hook for claude-deck integration
# This script receives hook events from Claude Code and updates the status file

STATUS_FILE="$HOME/.claude-deck/state.json"

# Ensure directory exists
mkdir -p "$(dirname "$STATUS_FILE")"

# Read JSON input from stdin
INPUT=$(cat)

# Extract event type and data using jq (if available) or basic parsing
if command -v jq &> /dev/null; then
    # Claude Code uses hook_event_name, not event
    EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // empty')
    TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
    MESSAGE=$(echo "$INPUT" | jq -r '.message // empty')

    # Extract tool detail based on tool type
    TOOL_DETAIL=""
    if [ -n "$TOOL_NAME" ]; then
        case "$TOOL_NAME" in
            "Read"|"Write"|"Edit")
                TOOL_DETAIL=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' | xargs basename 2>/dev/null)
                ;;
            "Bash")
                # Get first 30 chars of command
                TOOL_DETAIL=$(echo "$INPUT" | jq -r '.tool_input.command // empty' | head -c 100)
                ;;
            "Grep"|"Glob")
                TOOL_DETAIL=$(echo "$INPUT" | jq -r '.tool_input.pattern // empty' | head -c 80)
                ;;
            "Task")
                TOOL_DETAIL=$(echo "$INPUT" | jq -r '.tool_input.description // empty' | head -c 80)
                ;;
            "WebFetch"|"WebSearch")
                TOOL_DETAIL=$(echo "$INPUT" | jq -r '.tool_input.url // .tool_input.query // empty' | head -c 80)
                ;;
        esac
    fi

    # Try to get model from hook input first (most accurate)
    MODEL=$(echo "$INPUT" | jq -r '.model // empty')

    # Fall back to settings file if not in hook input
    if [ -z "$MODEL" ] || [ "$MODEL" = "null" ]; then
        SETTINGS_FILE="$HOME/.claude/settings.json"
        if [ -f "$SETTINGS_FILE" ]; then
            MODEL=$(jq -r '.model // empty' "$SETTINGS_FILE")
        else
            MODEL=""
        fi
    fi

    # Also check project-level settings
    if [ -z "$MODEL" ] || [ "$MODEL" = "null" ]; then
        PROJECT_SETTINGS=".claude/settings.json"
        if [ -f "$PROJECT_SETTINGS" ]; then
            MODEL=$(jq -r '.model // empty' "$PROJECT_SETTINGS")
        fi
    fi
else
    # Fallback: basic grep parsing
    EVENT=$(echo "$INPUT" | grep -o '"hook_event_name":"[^"]*"' | cut -d'"' -f4)
    TOOL_NAME=$(echo "$INPUT" | grep -o '"tool_name":"[^"]*"' | cut -d'"' -f4)
    MESSAGE=$(echo "$INPUT" | grep -o '"message":"[^"]*"' | cut -d'"' -f4)
    TOOL_DETAIL=""
    MODEL=""
fi

TIMESTAMP=$(date +%s)

# Determine task and state based on event type
case "$EVENT" in
    "UserPromptSubmit")
        TASK="THINKING"
        TOOL_DETAIL=""
        PROCESSING="true"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    "PreToolUse")
        TASK="$TOOL_NAME"
        # TOOL_DETAIL already extracted above
        PROCESSING="true"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    "PostToolUse")
        TASK="$TOOL_NAME"
        # TOOL_DETAIL already extracted above
        PROCESSING="true"
        WAITING="false"
        INPUT_TYPE="null"
        ;;
    "Notification")
        # Check if it's a permission request
        if echo "$MESSAGE" | grep -qi "permission\|approve\|allow\|confirm"; then
            TASK="PERMISSION"
            TOOL_DETAIL=""
            PROCESSING="false"
            WAITING="true"
            INPUT_TYPE='"permission"'
        elif echo "$MESSAGE" | grep -qi "error\|failed"; then
            TASK="ERROR"
            TOOL_DETAIL=""
            PROCESSING="false"
            WAITING="false"
            INPUT_TYPE="null"
        else
            TASK="$MESSAGE"
            TOOL_DETAIL=""
            PROCESSING="true"
            WAITING="false"
            INPUT_TYPE="null"
        fi
        ;;
    "Stop")
        TASK="READY"
        TOOL_DETAIL=""
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

# Handle tool_detail JSON - sanitize control characters and escape properly
if [ -z "$TOOL_DETAIL" ]; then
    TOOL_DETAIL_JSON="null"
else
    # Remove control characters, truncate, and use jq to properly escape for JSON
    TOOL_DETAIL=$(echo "$TOOL_DETAIL" | tr -d '\000-\037' | cut -c1-100)
    if [ -n "$TOOL_DETAIL" ]; then
        TOOL_DETAIL_JSON=$(echo -n "$TOOL_DETAIL" | jq -Rs '.')
    else
        TOOL_DETAIL_JSON="null"
    fi
fi

# Sanitize and truncate task (allow longer names for display)
TASK=$(echo "$TASK" | tr -d '\000-\037' | cut -c1-50)

# Write status file
cat > "$STATUS_FILE" << EOF
{
  "task": "$TASK",
  "tool_detail": $TOOL_DETAIL_JSON,
  "waiting_for_input": $WAITING,
  "input_type": $INPUT_TYPE,
  "model": $MODEL_JSON,
  "processing": $PROCESSING,
  "error": null,
  "timestamp": $TIMESTAMP
}
EOF

exit 0
