#!/bin/bash
# Template: Authenticated Session Workflow
# Purpose: Login once, persist auth, reuse for subsequent runs
# Usage: ./authenticated-session.sh <login-url> [runtime-profile]
#
# RECOMMENDED default: use a managed runtime profile for recurring authenticated work:
#   agent-browser --runtime-profile myapp runtime login <login-url>
#   agent-browser --runtime-profile myapp open <app-url>
#
# Alternative: use the auth vault when credential-driven login is appropriate:
#   echo "<pass>" | agent-browser auth save myapp --url <login-url> --username <user> --password-stdin
#   agent-browser auth login myapp
#
# Environment variables:
#   APP_USERNAME - Login username/email
#   APP_PASSWORD - Login password
#   RUNTIME_PROFILE - Override runtime profile name
#
# Two modes:
#   1. Discovery mode (default): Shows form structure so you can identify refs
#   2. Login mode: Performs actual login after you update the refs
#
# Setup steps:
#   1. Run once to see form structure (discovery mode)
#   2. Update refs in LOGIN FLOW section below
#   3. Set APP_USERNAME and APP_PASSWORD
#   4. Delete the DISCOVERY section

set -euo pipefail

LOGIN_URL="${1:?Usage: $0 <login-url> [runtime-profile]}"
RUNTIME_PROFILE="${RUNTIME_PROFILE:-${2:-default}}"

echo "Authentication workflow: $LOGIN_URL"
echo "Runtime profile: $RUNTIME_PROFILE"

# ================================================================
# RUNTIME PROFILE: Reuse existing authenticated profile when possible
# ================================================================
if agent-browser --runtime-profile "$RUNTIME_PROFILE" open "$LOGIN_URL" 2>/dev/null; then
    CURRENT_URL=$(agent-browser --runtime-profile "$RUNTIME_PROFILE" get url)
    if [[ "$CURRENT_URL" != *"login"* ]] && [[ "$CURRENT_URL" != *"signin"* ]]; then
        echo "Runtime profile already appears authenticated"
        agent-browser --runtime-profile "$RUNTIME_PROFILE" snapshot -i
        exit 0
    fi
    echo "Runtime profile needs sign-in or re-authentication"
    agent-browser --runtime-profile "$RUNTIME_PROFILE" close 2>/dev/null || true
fi

# ================================================================
# DISCOVERY MODE: Shows form structure (delete after setup)
# ================================================================
echo "Opening login page with detached runtime login..."
agent-browser --runtime-profile "$RUNTIME_PROFILE" runtime login "$LOGIN_URL"

echo ""
echo "Login form structure:"
echo "---"
agent-browser --runtime-profile "$RUNTIME_PROFILE" runtime status
echo "---"
echo ""
echo "Next steps:"
echo "  1. Complete manual login in the opened browser window"
echo "  2. Close the browser after sign-in"
echo "  3. Re-run with:"
echo "     agent-browser --runtime-profile \"$RUNTIME_PROFILE\" open <app-url>"
echo "  4. For Google or similar SSO, relaunch with --attachable only after sign-in"
echo ""
exit 0

# ================================================================
# LOGIN FLOW: Uncomment only if you intentionally want scripted credential entry
# ================================================================
# : "${APP_USERNAME:?Set APP_USERNAME environment variable}"
# : "${APP_PASSWORD:?Set APP_PASSWORD environment variable}"
#
# agent-browser --runtime-profile "$RUNTIME_PROFILE" open "$LOGIN_URL"
# agent-browser --runtime-profile "$RUNTIME_PROFILE" snapshot -i
#
# # Fill credentials (update refs to match your form)
# agent-browser --runtime-profile "$RUNTIME_PROFILE" fill @e1 "$APP_USERNAME"
# agent-browser --runtime-profile "$RUNTIME_PROFILE" fill @e2 "$APP_PASSWORD"
# agent-browser --runtime-profile "$RUNTIME_PROFILE" click @e3
# agent-browser --runtime-profile "$RUNTIME_PROFILE" wait --url "**/dashboard"
#
# # Verify login succeeded
# FINAL_URL=$(agent-browser --runtime-profile "$RUNTIME_PROFILE" get url)
# if [[ "$FINAL_URL" == *"login"* ]] || [[ "$FINAL_URL" == *"signin"* ]]; then
#     echo "Login failed - still on login page"
#     agent-browser --runtime-profile "$RUNTIME_PROFILE" screenshot /tmp/login-failed.png
#     agent-browser --runtime-profile "$RUNTIME_PROFILE" close
#     exit 1
# fi
#
# echo "Login successful"
# agent-browser --runtime-profile "$RUNTIME_PROFILE" snapshot -i
