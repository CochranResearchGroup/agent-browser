#!/bin/bash
# Template: Form Automation Workflow
# Purpose: Fill and submit web forms with validation
# Usage: ./form-automation.sh <form-url>
#
# This template demonstrates the snapshot-interact-verify pattern:
# 1. Navigate to form
# 2. Snapshot to get element refs
# 3. Fill fields using refs
# 4. Submit and verify result
#
# Customize: Update the refs (@e1, @e2, etc.) based on your form's snapshot output
# Optional: set RUNTIME_PROFILE=<name> to run against a managed runtime profile

set -euo pipefail

FORM_URL="${1:?Usage: $0 <form-url>}"
RUNTIME_PROFILE="${RUNTIME_PROFILE:-}"
AB=(agent-browser)
if [[ -n "$RUNTIME_PROFILE" ]]; then
    AB+=(--runtime-profile "$RUNTIME_PROFILE")
fi

echo "Form automation: $FORM_URL"

# Step 1: Navigate to form
"${AB[@]}" open "$FORM_URL"

# Step 2: Snapshot to discover form elements
echo ""
echo "Form structure:"
"${AB[@]}" snapshot -i

# Step 3: Fill form fields (customize these refs based on snapshot output)
#
# Common field types:
#   agent-browser fill @e1 "John Doe"           # Text input
#   agent-browser fill @e2 "user@example.com"   # Email input
#   agent-browser fill @e3 "SecureP@ss123"      # Password input
#   agent-browser select @e4 "Option Value"     # Dropdown
#   agent-browser check @e5                     # Checkbox
#   agent-browser click @e6                     # Radio button
#   agent-browser fill @e7 "Multi-line text"   # Textarea
#   agent-browser upload @e8 /path/to/file.pdf # File upload
#
# Uncomment and modify:
# "${AB[@]}" fill @e1 "Test User"
# "${AB[@]}" fill @e2 "test@example.com"
# "${AB[@]}" click @e3  # Submit button

# Step 4: Wait for submission
# "${AB[@]}" wait --url "**/success"  # Or wait for redirect
# "${AB[@]}" wait 1000                # Optional short settle time for result UI

# Step 5: Verify result
echo ""
echo "Result:"
"${AB[@]}" get url
"${AB[@]}" snapshot -i

# Optional: Capture evidence
"${AB[@]}" screenshot /tmp/form-result.png
echo "Screenshot saved: /tmp/form-result.png"

# Cleanup
"${AB[@]}" close
echo "Done"
