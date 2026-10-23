#!/usr/bin/env python3
"""Provider-free public MCP dispatch regression with isolated durable task state."""
import json, os, select, socket, sqlite3, subprocess, sys, tempfile, threading
from pathlib import Path

binary = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="ab365-task-") as directory:
    root = Path(directory)
    service = root / ".agent-browser/service"
    service.mkdir(parents=True)
    lane = "workflow-duplicate"
    tasks = {}
    for profile in ("profile-a", "profile-b"):
        task_id = f"session:{lane}:{profile}:1"
        tasks[task_id] = dict(id=task_id, name=lane, profileId=profile,
            browserId=f"browser:{profile}", createdAtMs=1, lastActivityAtMs=1,
            expiresAtMs=9999999999999, currentTabId=None)
    schema = "agent-browser.browser-session-state.v1"
    connection = sqlite3.connect(service / "runtime.sqlite3")
    connection.executescript("PRAGMA user_version=1; CREATE TABLE state_documents(kind TEXT PRIMARY KEY, schema_version TEXT NOT NULL, json TEXT NOT NULL);")
    connection.execute("INSERT INTO state_documents VALUES (?,?,?)",
        ("browser_session_state", schema, json.dumps(dict(schemaVersion=schema, sessions=tasks))))
    connection.commit()
    connection.close()
    listener = socket.socket(socket.AF_UNIX)
    listener.bind(str(root / (lane + ".sock")))
    listener.listen()
    listener.settimeout(0.1)
    (root / (lane + ".token")).write_text("synthetic-fixture-token")
    (root / "empty.env").write_text("")
    received = []
    stopped = threading.Event()
    def serve():
        while not stopped.is_set():
            try:
                peer, _ = listener.accept()
            except socket.timeout:
                continue
            with peer:
                request = json.loads(peer.makefile("rb").readline())
                received.append(request)
                exact = request.get("sessionId")
                ok = exact in tasks
                peer.sendall((json.dumps(dict(id=request.get("id"), success=ok,
                    data=dict(addressedTask=exact) if ok else None,
                    error=None if ok else "browser_session_name_ambiguous:" + lane)) + "\n").encode())
    worker = threading.Thread(target=serve, daemon=True)
    worker.start()
    env = {k:v for k,v in os.environ.items() if not k.startswith("AGENT_BROWSER_")}
    env.update(HOME=str(root), XDG_CONFIG_HOME=str(root / "config"),
        AGENT_BROWSER_SOCKET_DIR=str(root), AGENT_BROWSER_RUNTIME_HOST="0",
        AGENT_BROWSER_ENV_FILE=str(root / "empty.env"))
    process = subprocess.Popen([str(binary), "mcp", "serve"], env=env,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        cases = [
            ("browser_snapshot", {}),
            ("browser_navigate", {"url":"https://synthetic.example/"}),
            ("browser_click", {"selector":"#synthetic"}),
            ("browser_fill", {"selector":"#synthetic", "value":"fixture"}),
            ("browser_screenshot", {}),
            ("browser_command", {"action":"url"}),
            ("browser_command", {"action":"evaluate", "params":{"script":"1"}}),
        ]
        index = 0
        for task_id, (tool, arguments) in [(task, case) for task in tasks for case in cases]:
            index += 1
            message = dict(jsonrpc="2.0", id=index, method="tools/call",
                params=dict(name=tool, arguments=dict(taskId=task_id, **arguments)))
            process.stdin.write(json.dumps(message) + "\n")
            process.stdin.flush()
            assert select.select([process.stdout], [], [], 5)[0], "MCP response timeout"
            response = json.loads(process.stdout.readline())
            assert "error" not in response, response
            payload = json.loads(response["result"]["content"][0]["text"])
            assert payload["success"] is True, payload
            assert received, response
            assert received[-1].get("sessionId") == task_id, ("exact task identity lost", {k:v for k,v in received[-1].items() if k != "_agentBrowserAuthToken"})
            assert received[-1].get("profileId") == tasks[task_id]["profileId"]
            assert "browser_session_name_ambiguous" not in json.dumps(response), response
        before = len(received)
        process.stdin.write(json.dumps(dict(jsonrpc="2.0", id=999, method="tools/call",
            params=dict(name="browser_snapshot", arguments=dict(taskId="session:missing")))) + "\n")
        process.stdin.flush()
        assert select.select([process.stdout], [], [], 5)[0], "MCP rejection timeout"
        rejected = json.loads(process.stdout.readline())
        assert "browser_task_not_found" in json.dumps(rejected), rejected
        assert len(received) == before, "missing task must fail before transport dispatch"
        print("PASS: public MCP page actions preserve each same-name task's exact identity")
    finally:
        process.terminate()
        process.communicate(timeout=5)
        stopped.set()
        worker.join(timeout=2)
        listener.close()
