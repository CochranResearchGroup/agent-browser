#!/bin/bash
# Template: Content Capture Workflow
# Purpose: Extract content from web pages (text, screenshots, PDF)
# Usage: ./capture-workflow.sh <url> [output-dir]
#
# Outputs:
#   - page-full.png: Full page screenshot
#   - page-structure.txt: Page element structure with refs
#   - page-text.txt: All text content
#   - page.pdf: PDF version
#
# Optional: use a runtime profile for protected pages
#   RUNTIME_PROFILE=work ./capture-workflow.sh https://app.example.com/dashboard

set -euo pipefail

TARGET_URL="${1:?Usage: $0 <url> [output-dir]}"
OUTPUT_DIR="${2:-.}"
RUNTIME_PROFILE="${RUNTIME_PROFILE:-}"
AB=(agent-browser)
if [[ -n "$RUNTIME_PROFILE" ]]; then
    AB+=(--runtime-profile "$RUNTIME_PROFILE")
fi

echo "Capturing: $TARGET_URL"
mkdir -p "$OUTPUT_DIR"

# Navigate to target
"${AB[@]}" open "$TARGET_URL"

# Get metadata
TITLE=$("${AB[@]}" get title)
URL=$("${AB[@]}" get url)
echo "Title: $TITLE"
echo "URL: $URL"

# Capture full page screenshot
"${AB[@]}" screenshot --full "$OUTPUT_DIR/page-full.png"
echo "Saved: $OUTPUT_DIR/page-full.png"

# Get page structure with refs
"${AB[@]}" snapshot -i > "$OUTPUT_DIR/page-structure.txt"
echo "Saved: $OUTPUT_DIR/page-structure.txt"

# Extract all text content
"${AB[@]}" get text body > "$OUTPUT_DIR/page-text.txt"
echo "Saved: $OUTPUT_DIR/page-text.txt"

# Save as PDF
"${AB[@]}" pdf "$OUTPUT_DIR/page.pdf"
echo "Saved: $OUTPUT_DIR/page.pdf"

# Optional: Extract specific elements using refs from structure
# "${AB[@]}" get text @e5 > "$OUTPUT_DIR/main-content.txt"

# Optional: Handle infinite scroll pages
# for i in {1..5}; do
#     "${AB[@]}" scroll down 1000
#     "${AB[@]}" wait 1000
# done
# "${AB[@]}" screenshot --full "$OUTPUT_DIR/page-scrolled.png"

# Cleanup
"${AB[@]}" close

echo ""
echo "Capture complete:"
ls -la "$OUTPUT_DIR"
