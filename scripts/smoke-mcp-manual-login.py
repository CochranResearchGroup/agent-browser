#!/usr/bin/env python3
"""Public MCP + real Chrome/X11; synthetic r3 provider, no live accounts/viewer."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--chrome', required=True)
    parser.add_argument('--namespace-worker', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--parent-proof-fd', type=int, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not args.namespace_worker:
        read_fd, write_fd = os.pipe()
        proof = dict(parentPid=os.getpid(), namespaces={key: os.readlink('/proc/self/ns/' + key) for key in ['user', 'mnt', 'net']})
        os.write(write_fd, json.dumps(proof).encode())
        os.close(write_fd)
        try:
            result = subprocess.run(['unshare', '--user', '--map-current-user', '--keep-caps', '--mount', '--net',
                '--propagation', 'private', 'python3', str(Path(__file__).resolve()),
                '--binary', str(Path(args.binary).resolve()), '--chrome', str(Path(args.chrome).resolve()),
                '--namespace-worker', '--parent-proof-fd', str(read_fd)], pass_fds=(read_fd,)).returncode
        finally:
            os.close(read_fd)
        raise SystemExit(result)
    assert args.parent_proof_fd is not None, 'private namespace proof required'
    with os.fdopen(args.parent_proof_fd) as source:
        proof = json.load(source)
    assert proof['parentPid'] == os.getppid(), 'fixture parent changed'
    for key, previous in proof['namespaces'].items():
        assert os.readlink('/proc/self/ns/' + key) != previous, 'private namespace required: ' + key

    wire = json.loads((Path(__file__).resolve().parents[1] / 'docs/dev/contracts/remote-view-application-r3.v1.fixture.json').read_text())
    with tempfile.TemporaryDirectory(prefix='ab357-mcp-') as temporary:
        root = Path(temporary)
        sockets = root / 'x11-sockets'
        sockets.mkdir(mode=0o1777)
        sockets.chmod(0o1777)
        subprocess.run(['mount', '--bind', str(sockets), '/tmp/.X11-unix'], check=True)
        subprocess.run(['ip', 'link', 'set', 'lo', 'up'], check=True)

        env = {k: v for k, v in os.environ.items() if not k.startswith(('AGENT_BROWSER_', 'XDG_'))}
        for k in ['DISPLAY', 'XAUTHORITY', 'DBUS_SESSION_BUS_ADDRESS']:
            env.pop(k, None)
        env.update(HOME=temporary, AGENT_BROWSER_HOME=str(root / '.agent-browser'),
                   AGENT_BROWSER_SOCKET_DIR=str(root / 's'), AGENT_BROWSER_RUNTIME_HOST='1',
                   AGENT_BROWSER_DEBUG='1',
                   AGENT_BROWSER_EXECUTABLE_PATH=str(Path(args.chrome).resolve()),
                   AGENT_BROWSER_PUBLIC_ORIGIN='https://browser.fixture.invalid',
                   AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN='https://native.fixture.invalid',
                   AGENT_BROWSER_REMOTE_VIEW_POOL='main', AGENT_BROWSER_REMOTE_VIEW_APPLICATION='agent-browser',
                   AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT='1',
                   AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS='0', AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS='0')
        for key in ['XDG_RUNTIME_DIR', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_CACHE_HOME']:
            env[key] = str(root / key.lower())
            Path(env[key]).mkdir(mode=0o700)
        Path(env['AGENT_BROWSER_SOCKET_DIR']).mkdir(mode=0o700)
        (root / 'Xauthority').touch(mode=0o600)
        env['AGENT_BROWSER_HOSTS_FILE'] = str(root / 'hosts.json')
        (root / 'hosts.json').write_text(json.dumps(dict(version=1, hosts={})))
        children, registered, operations = [], {}, []
        cookie_seeded = threading.Event()
        cookie_after_close = threading.Event()
        cookie_restored = threading.Event()
        get_requests = []
        def spawn(command, **kwargs):
            child = subprocess.Popen(command, env=env, start_new_session=True, **kwargs)
            children.append(child)
            return child
        def run(command):
            return subprocess.run(command, env=env, capture_output=True, text=True, timeout=3)
        def browsers():
            found = []
            for path in Path('/proc').iterdir():
                if not path.name.isdecimal():
                    continue
                try:
                    cmd = (path / 'cmdline').read_bytes().replace(bytes([0]), b' ').decode()
                    if '--user-data-dir=' + str(root) in cmd and ' --type=' not in cmd:
                        found.append((int(path.name), cmd))
                except (OSError, UnicodeError):
                    pass
            return found
        def windows():
            result = []
            active = run(['xdotool', 'getactivewindow']).stdout.strip()
            for pid, _ in browsers():
                for window in run(['xdotool', 'search', '--onlyvisible', '--pid', str(pid)]).stdout.split():
                    geometry = run(['xdotool', 'getwindowgeometry', '--shell', window])
                    values = dict(re.findall(r'^(X|Y|WIDTH|HEIGHT)=(-?\d+)$', geometry.stdout, re.M))
                    if len(values) == 4:
                        result.append(dict(id=int(window), title=run(['xdotool', 'getwindowname', window]).stdout.strip(), **{'class': 'chrome'},
                                           pid=pid, x=int(values['X']), y=int(values['Y']),
                                           width=int(values['WIDTH']), height=int(values['HEIGHT']),
                                           active=window == active, maximized=False))
            return result
        class Provider(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass
            def do_GET(self):
                if self.path not in ['/login-fixture', '/cookie-check']:
                    self.send_response(404)
                    self.end_headers()
                    return
                cookie_present = 'ab357_session=synthetic' in self.headers.get('Cookie', '').split('; ')
                get_requests.append(dict(path=self.path, cookiePresent=cookie_present, afterClose=cookie_after_close.is_set()))
                if cookie_present:
                    if cookie_after_close.is_set():
                        if self.path == '/login-fixture':
                            cookie_restored.set()
                    elif self.path == '/cookie-check':
                        cookie_seeded.set()
                self.send_response(200)
                if self.path == '/login-fixture' and not cookie_after_close.is_set():
                    self.send_header('Set-Cookie', 'ab357_session=synthetic; Max-Age=3600; Path=/; HttpOnly; SameSite=Lax')
                self.send_header('Content-Type', 'text/html')
                self.end_headers()
                self.wfile.write(b"<!doctype html><title>Manual login fixture</title><script>fetch('/cookie-check')</script>")
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                request = body['request']
                operation = request['operation']
                operations.append(operation)
                value = None
                if operation == 'inventory':
                    value = copy.deepcopy(wire['inventory'])
                    value['policy']['pools']['main']['maximum_reserved'] = 1
                elif operation == 'observe_assignment':
                    value = wire['assignmentObservation']
                elif operation == 'launch_environment':
                    value = copy.deepcopy(wire['launchEnvironment'])
                    value['environment'] = {'DISPLAY': env['DISPLAY'], 'XAUTHORITY': str(root / 'Xauthority'),
                                            'REMOTE_VIEW_SLOT_GENERATION': str(wire['assignmentObservation']['target']['viewingGeneration'])}
                elif operation == 'windows':
                    value = copy.deepcopy(wire['windows'])
                    value['windows'] = windows()
                elif operation == 'register_native_handoff_observation':
                    key = request['idempotencyKey']
                    application = body['application'].encode()
                    digest = hashlib.sha256(len(application).to_bytes(8, 'big') + application + key.encode()).hexdigest()
                    registered[key] = dict(correlationId=hashlib.sha256(key.encode()).hexdigest(),
                                           requestDigest=digest, application=body['application'],
                                           target=wire['assignmentObservation']['target'], targetDigest=request['targetDigest'],
                                           clock=dict(bootId='synthetic-boot', elapsedMs=100), lifetimeMs=request['lifetimeMs'], used=None)
                    value = dict(schemaVersion=1, observation=registered[key])
                elif operation == 'observe_native_handoff_use':
                    record = next(v for v in registered.values() if v['correlationId'] == request['correlationId'])
                    value = dict(schemaVersion=1, state='pending', observation=record)
                elif operation == 'read_idle_retirement':
                    value = dict(schemaVersion=1, target=wire['assignmentObservation']['target'],
                                 readinessScope='native_admission', retirement=dict(revision=0, fence=None, completed=True))
                if value is None:
                    self.send_response(400)
                    self.end_headers()
                    self.wfile.write(b'{"code":"unsupported_fixture_operation"}')
                    return
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(value).encode())
        provider = ThreadingHTTPServer(('127.0.0.1', 0), Provider)
        threading.Thread(target=provider.serve_forever, daemon=True).start()
        env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN'] = 'http://127.0.0.1:' + str(provider.server_port)
        mcp, task = None, None
        seq = 0
        def rpc(method, params):
            nonlocal seq
            seq += 1
            mcp.stdin.write(json.dumps(dict(jsonrpc='2.0', id=seq, method=method, params=params)) + '\n')
            mcp.stdin.flush()
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                assert select.select([mcp.stdout], [], [], deadline - time.monotonic())[0], 'MCP response deadline'
                line = mcp.stdout.readline()
                assert line, 'MCP exited before response'
                reply = json.loads(line)
                if reply.get('id') == seq:
                    assert 'error' not in reply, reply.get('error')
                    return reply['result']
            raise AssertionError('MCP response deadline')
        def call(name, arguments):
            result = rpc('tools/call', dict(name=name, arguments=arguments))
            assert not result.get('isError'), result
            payload = json.loads(result['content'][0]['text'])
            assert payload['success'], payload
            return payload['data']
        try:
            read_fd, write_fd = os.pipe()
            try:
                with (root / 'xvfb.log').open('w') as diagnostics:
                    xvfb = spawn(['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '1280x800x24', '-nolisten', 'tcp'],
                                 pass_fds=(write_fd,), stdout=subprocess.DEVNULL, stderr=diagnostics)
                os.close(write_fd)
                assert select.select([read_fd], [], [], 10)[0], 'Xvfb readiness deadline'
                display_number = os.read(read_fd, 64).decode().strip()
                assert display_number.isdecimal() and xvfb.poll() is None, 'Xvfb exited before publishing a display'
                env['DISPLAY'] = ':' + display_number
            finally:
                os.close(read_fd)
            deadline = time.monotonic() + 10
            while run(['xprop', '-root']).returncode != 0:
                assert xvfb.poll() is None, 'Xvfb exited before accepting X11 clients'
                assert time.monotonic() < deadline, 'X11 client readiness deadline'
                time.sleep(.02)
            spawn(['openbox', '--sm-disable'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            deadline = time.monotonic() + 10
            while 'window id #' not in run(['xprop', '-root', '_NET_SUPPORTING_WM_CHECK']).stdout:
                assert time.monotonic() < deadline, 'window manager readiness deadline'
                time.sleep(.02)
            created = run([str(Path(args.binary).resolve()), 'runtime', 'create', 'manual-fixture', '--json'])
            assert created.returncode == 0, created.stderr
            profile = Path(json.loads(created.stdout)['userDataDir'])
            profile.mkdir(parents=True, exist_ok=True)
            # Model a human's completed browser first-run setup, not a login.
            # The real cookie is then seeded over HTTP without CDP or DB edits.
            (profile / 'First Run').touch()
            mcp_log = root / 'mcp.log'
            with mcp_log.open('w') as diagnostics:
                mcp = spawn([str(Path(args.binary).resolve()), 'mcp', 'serve'], stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=diagnostics, text=True)
            rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='ab357-fixture', version='1')))
            mcp.stdin.write(json.dumps(dict(jsonrpc='2.0', method='notifications/initialized')) + '\n')
            mcp.stdin.flush()
            request = dict(profile='manual-fixture', profileMode='persistent', mode='manual_login', taskKey='google-login',
                           url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN'] + '/login-fixture',
                           serviceName='fixture', agentName='fixture', taskName='Manual login')
            task = call('browser_open', request)
            assert task['mode'] == 'manual_login' and task['urlObserved'] is False
            assert task['operatorVisible']['state'] == 'ready'
            assert task['handoffUrl'].startswith('https://browser.fixture.invalid/remote-view/')
            physical = browsers()
            assert len(physical) == 1, 'one exact browser process'
            pid, argv = physical[0]
            assert '--remote-debugging' not in argv, 'manual login must have no debugging port or pipe'
            assert '--headless' not in argv, 'manual login must be headed'
            replay = call('browser_open', request)
            assert (replay['taskId'], replay['browserId'], replay['handoffUrl']) == (task['taskId'], task['browserId'], task['handoffUrl'])
            assert browsers()[0][0] == pid, 'replay preserves the process'
            try:
                call('browser_snapshot', dict(taskId=task['taskId']))
                raise AssertionError('manual login accepted DOM automation')
            except AssertionError as error:
                assert 'manual_login_automation_unavailable' in str(error), str(error)
            assert cookie_seeded.wait(10), 'manual browser must accept the synthetic persistent cookie'
            call('browser_close', dict(taskId=task['taskId']))
            task = None
            assert not browsers(), 'explicit close must remove the exact browser'
            cookie_after_close.set()
            profiles = list(root.rglob('manual-fixture/user-data'))
            assert len(profiles) == 1, 'persistent profile survives close'
            marker = profiles[0] / 'ab357-preservation-marker'
            marker.write_text('synthetic profile continuity')
            automation = dict(request, mode='automation', taskKey='automation')
            task = call('browser_open', automation)
            assert len(browsers()) == 1 and '--remote-debugging-port=' in browsers()[0][1]
            assert marker.read_text() == 'synthetic profile continuity'
            assert cookie_restored.wait(10), 'persistent cookie must survive manual close and automation reopen'
            call('browser_close', dict(taskId=task['taskId']))
            task = None
            assert not browsers(), 'automation close must remove the exact browser'

        except BaseException:
            # This namespace contains synthetic data only. Keep bounded failure
            # evidence before its disposable filesystem is removed.
            logs = {str(path.relative_to(root)): path.read_text(errors='replace')[-6000:]
                    for path in [root / 'mcp.log', root / 'xvfb.log', root / 's/runtime-host.log',
                                 root / 'xdg_cache_home/openbox/openbox.log'] if path.is_file()}
            print(json.dumps(dict(fixtureFailure=True, operations=operations, getRequests=get_requests,
                                  windows=windows(), logs=logs)), flush=True)
            raise
        finally:
            # Only children and processes carrying this exact disposable namespace.
            if mcp and mcp.poll() is None:
                mcp.terminate()
            tag = ('AGENT_BROWSER_HOME=' + env['AGENT_BROWSER_HOME']).encode()
            for path in Path('/proc').iterdir():
                if not path.name.isdecimal() or int(path.name) == os.getpid():
                    continue
                try:
                    if tag in (path / 'environ').read_bytes().split(bytes([0])):
                        os.kill(int(path.name), signal.SIGTERM)
                except OSError:
                    pass
            for child in reversed(children):
                if child.poll() is None:
                    child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
            provider.shutdown()
            provider.server_close()
            deadline = time.monotonic() + 5
            while True:
                residue = []
                for path in Path('/proc').iterdir():
                    if not path.name.isdecimal() or int(path.name) == os.getpid():
                        continue
                    try:
                        if tag in (path / 'environ').read_bytes().split(bytes([0])):
                            residue.append(int(path.name))
                    except OSError:
                        pass
                if not residue:
                    break
                assert time.monotonic() < deadline, 'fixture process residue: ' + str(residue)
                time.sleep(.02)
        print(json.dumps(dict(passed=True, scope='public MCP, real Chrome/X11, synthetic r3 provider; no live Google or viewer',
                              noCdp=True, headed=True, readyHandoff=True, replayPreserved=True, domRefused=True,
                              closed=True, profilePreserved=True, persistentCookiePreserved=True,
                              automationTransition=True, cleanupVerified=True)))


if __name__ == '__main__':
    main()
