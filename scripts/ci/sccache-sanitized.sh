#!/usr/bin/env bash
set -euo pipefail

sccache_executable="${AGENT_BROWSER_SCCACHE_EXECUTABLE:-}"
if [[ -z "$sccache_executable" || ! -x "$sccache_executable" ]]; then
  echo "Cargo cache wrapper requires an executable AGENT_BROWSER_SCCACHE_EXECUTABLE" >&2
  exit 78
fi

# sccache includes its complete inherited environment when some process-spawn
# failures are formatted with Debug. Rust compilation does not need unrelated
# application credentials, so remove credential-shaped variables at this
# boundary while leaving Cargo and test-process environments unchanged.
while IFS= read -r variable_name; do
  case "$variable_name" in
    *_API_KEY|*_APP_TOKEN|*_AUTH|*_AUTH_TOKEN|*_BOT_TOKEN|*_CLIENT_SECRET|*_CREDENTIAL|*_CREDENTIALS|*_KEY|*_PASS|*_PASSPHRASE|*_PASSWORD|*_PRIVATE_KEY|*_REFRESH|*_REFRESH_TOKEN|*_SECRET|*_SIGNING_SECRET|*_TOKEN|*_USER_TOKEN)
      unset "$variable_name"
      ;;
  esac
done < <(compgen -e)
unset AGENT_BROWSER_SCCACHE_EXECUTABLE

exec "$sccache_executable" "$@"
