#!/usr/bin/env bash
set -euo pipefail

readonly template_source="${GUACAMOLE_HOME:?GUACAMOLE_HOME must name the packaged template}"
readonly writable_template="/tmp/agent-browser-guacamole-template"

rm -rf "$writable_template"
mkdir -p "$writable_template"
cp -R "$template_source/." "$writable_template/"
chmod -R u+rwX "$writable_template"
export GUACAMOLE_HOME="$writable_template"

exec /opt/guacamole/bin/start.sh
