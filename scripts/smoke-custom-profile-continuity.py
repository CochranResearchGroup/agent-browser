#!/usr/bin/env python3
"""Opt-in browser regression: python3 scripts/smoke-custom-profile-continuity.py BINARY.

Runs only synthetic pages with a disposable HOME and an explicitly owned host.
Retains private receipts and profile data under the printed temporary directory.
"""

import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time

binary = Path(sys.argv[1]).resolve(strict=True)
root = Path(tempfile.mkdtemp(prefix="p159-profile-repro-"))
print(root, flush=True)
env = {k: v for k, v in os.environ.items() if not k.startswith("AGENT_BROWSER_")}
env.update(
    HOME=str(root),
    XDG_CONFIG_HOME=str(root / ".config"),
    AGENT_BROWSER_HOME=str(root / ".agent-browser"),
    AGENT_BROWSER_SOCKET_DIR=str(root / "s"),
    AGENT_BROWSER_RUNTIME_HOST="1",
    AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS="0",
    AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY="disabled",
    AGENT_BROWSER_EXECUTABLE_PATH="/opt/google/chrome/chrome",
    AGENT_BROWSER_IDLE_TIMEOUT_MS="60000",
    AGENT_BROWSER_SESSION_SUPERVISOR_ROOT=str(root / "supervisor"),
)
(root / "s").mkdir()
manifest_dir = root / "supervisor/manifests"
manifest_dir.mkdir(parents=True)
for session in ["profile-test", "keeper"]:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    manifest = {
        "schemaVersion": "agent-browser.session-supervisor.v1",
        "session": session,
        "executablePath": str(binary),
        "executableSha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "streamPort": port,
        "provenance": {
            "packageVersion": "0.28.0",
            "installedAt": "2026-09-06T17:00:00Z",
            "installedBy": "isolated custom profile regression",
        },
    }
    (manifest_dir / (session + ".json")).write_text(json.dumps(manifest))
token = os.urandom(32).hex()
(root / "s/runtime-host.token").write_text(token)
(root / "s/runtime-host.token").chmod(0o600)
env["AGENT_BROWSER_DAEMON_AUTH_TOKEN"] = token
results = []
browser_identities = {}


def run(step, args, session="profile-test", profile=True):
    command = [str(binary), "--session", session, "--json"]
    if profile:
        command += ["--profile", str(root / "profile")]
    response = subprocess.run(
        command + args, env=env, capture_output=True, text=True, timeout=45
    )
    result = {
        "step": step,
        "code": response.returncode,
        "out": response.stdout,
        "err": response.stderr,
    }
    results.append(result)
    print(step, response.returncode, response.stdout[:400], flush=True)
    state = root / ".agent-browser/service/state.json"
    if state.exists():
        (root / (step + "-state.json")).write_bytes(state.read_bytes())
        snapshot = json.loads(state.read_text())
        browser = snapshot.get("browsers", {}).get("session:profile-test")
        if browser and step in ["open", "url", "click", "read"]:
            browser_identities[step] = {key: browser.get(key) for key in ["pid", "profileId", "tabIds"]}
    return result


with (root / "host.log").open("w") as log:
    host = subprocess.Popen(
        [str(binary)],
        env=dict(env, AGENT_BROWSER_RUNTIME_HOST_PROCESS="1", AGENT_BROWSER_SESSION="runtime-host"),
        stdout=log,
        stderr=log,
    )
    try:
        deadline = time.monotonic() + 10
        while not (root / "s/runtime-host.sock").exists():
            if host.poll() is not None or time.monotonic() >= deadline:
                raise RuntimeError("Host did not become ready; inspect host.log")
            time.sleep(0.05)
        run("keeper", ["stream", "status"], session="keeper", profile=False)
        page = 'data:text/html,<title>profile test</title><button id="button" onclick="this.textContent=String(42)">test</button>'
        for step, args in [
            ("open", ["open", page]),
            ("url", ["get", "url"]),
            ("click", ["click", "#button"]),
            ("read", ["get", "text", "#button"]),
            ("close", ["close"]),
            ("reopen", ["open", page]),
            ("url-reopened", ["get", "url"]),
            ("close-final", ["close"]),
        ]:
            run(step, args)
    finally:
        try:
            run("cleanup", ["close"])
        finally:
            host.terminate()
            host.wait(timeout=15)
            (root / "results.json").write_text(json.dumps(results, indent=2))

failed = [result["step"] for result in results if result["code"] != 0]
if failed:
    raise SystemExit("Custom profile continuity failed: " + ", ".join(failed))
read = next(result for result in results if result["step"] == "read")
assert json.loads(read["out"])["data"]["text"] == "42", "Click did not change synthetic content"
assert browser_identities["open"]["pid"], "Browser PID missing"
assert all(identity == browser_identities["open"] for identity in browser_identities.values()), "Commands replaced the browser or target"
print("Custom profile continuity passed")
