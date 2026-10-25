#!/usr/bin/env python3
"""Public MCP + real Chrome/X11; synthetic r3 provider, no live accounts/viewer."""
import argparse
import base64
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import select
import signal
import shutil
import sqlite3
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--chrome', required=True)
    parser.add_argument('--mode-conversion', action='store_true')
    parser.add_argument('--recovery-case', choices=['pre-spawn-retry', 'pre-spawn-restore', 'native-restore', 'lost-reply', 'published-terminal'])
    parser.add_argument('--restart-owner', action='store_true')
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
                '--namespace-worker', '--parent-proof-fd', str(read_fd), *(['--mode-conversion'] if args.mode_conversion else []), *(['--recovery-case', args.recovery_case] if args.recovery_case else []), *(['--restart-owner'] if args.restart_owner else [])], pass_fds=(read_fd,)).returncode
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
        original_chrome = Path(args.chrome).resolve()
        private_bundle = root / 'chrome-build'
        private_bundle.mkdir()
        for resource in original_chrome.parent.iterdir():
            target = private_bundle / resource.name
            if resource == original_chrome:
                shutil.copy2(resource, target)
            else:
                target.symlink_to(resource, target_is_directory=resource.is_dir())
        args.chrome = str(private_bundle / original_chrome.name)
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
        inject_pre_spawn = threading.Event()
        suppress_replacement_windows = threading.Event()
        original_pid = None
        executable_pin_provenance = []
        native_page=threading.Event(); native_mouse=threading.Event(); native_scroll=threading.Event(); native_auth=threading.Event()
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
        def process_identity(pid):
            fields=Path('/proc/'+str(pid)+'/stat').read_text().rsplit(')',1)[1].split()
            return dict(pid=pid,start_token=fields[19],boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),executable=os.readlink('/proc/'+str(pid)+'/exe'))
        def assert_conversion_executable_pin(profile_id, mode):
            # Read only this synthetic owner's durable state after the public
            # conversion; no profile pin is seeded by the fixture itself.
            databases = list(root.rglob('runtime.sqlite3'))
            assert len(databases) == 1, 'one exact fixture runtime database required'
            with sqlite3.connect('file:' + str(databases[0]) + '?mode=ro', uri=True) as connection:
                row = connection.execute("SELECT json FROM state_documents WHERE kind='browser_session_state'").fetchone()
            assert row, 'converted owner state must be durable'
            state = json.loads(row[0])
            assert state.get('profileExecutables', {}).get(profile_id) == args.chrome, 'public conversion must retain the exact fixture executable pin'
            physical = browsers()
            assert len(physical) == 1, 'conversion must retain one exact browser process'
            assert process_identity(physical[0][0])['executable'] == args.chrome, 'converted process must use the retained executable pin'
            executable_pin_provenance.append(dict(mode=mode, profileId=profile_id, durablePinMatches=True, processExecutableMatches=True))
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
                                           active=window == active, maximized=False, processIdentity=process_identity(pid)))
            return result
        class Provider(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass
            def do_GET(self):
                if self.path in ['/native-login','/mouse-proof','/scroll-proof','/auth-check']:
                    if self.path=='/mouse-proof':native_mouse.set()
                    if self.path=='/scroll-proof':native_scroll.set()
                    if self.path=='/auth-check' and 'native_auth=verified' in self.headers.get('Cookie',''):native_auth.set()
                    self.send_response(200);self.send_header('Content-Type','text/html');self.end_headers()
                    if self.path=='/native-login':
                        self.wfile.write(b"<!doctype html><title>Native login</title><body style='height:3000px'><form method='POST' action='/native-submit'><input name='proof'><button>Sign in</button></form><script>document.addEventListener('click',()=>fetch('/mouse-proof'));window.addEventListener('scroll',()=>fetch('/scroll-proof'))</script></body>")
                        native_page.set()
                    else:self.wfile.write(b'<!doctype html><title>Native proof</title>Observed')
                    return
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
                if self.path=='/native-submit':
                    proof=self.rfile.read(int(self.headers['Content-Length']))
                    assert proof==b'proof=synthetic-proof', 'synthetic form submission independently verified'
                    self.send_response(303);self.send_header('Set-Cookie','native_auth=verified; Max-Age=3600; Path=/; HttpOnly; SameSite=Lax');self.send_header('Location','/auth-check');self.end_headers();return
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                if self.path.startswith('/v1/agent-desktop/'):
                    operation=self.path.rsplit('/',1)[1]
                    assert body['desktopId']==wire['assignmentObservation']['target']['viewingDesktopId'] and body['expectedGeneration']==19
                    current=windows()
                    if suppress_replacement_windows.is_set():
                        current = [window for window in current if window['pid'] == original_pid]
                    if operation=='observe':
                        value=dict(desktopId=body['desktopId'],generation=19,geometry=dict(current=dict(width=1280,height=800)),windows=dict(windows=current))
                    else:
                        window=next(w for w in current if w['id']==body['windowId'])
                        assert window['processIdentity']==body['expectedProcessIdentity'], 'Native process fence changed'
                        if operation=='window':
                            assert body['action']=='activate'; result=run(['xdotool','windowactivate','--sync',str(window['id'])]); assert result.returncode==0,result.stderr
                        elif operation=='input':
                            assert window['active'], 'Native input target must be active'
                            assert body['expectedWidth']==1280 and body['expectedHeight']==800
                            assert body['expectedWindow']=={key:window[key] for key in ['x','y','width','height']}
                            for action in body['actions']:
                                if action['kind']=='key':
                                    if action['chord'].lower() == 'alt+f4' and inject_pre_spawn.is_set():
                                        Path(args.chrome).chmod(0o600)
                                        inject_pre_spawn.clear()
                                    result=run(['xdotool','key','--clearmodifiers',action['chord']])
                                elif action['kind']=='text': result=run(['xdotool','type','--clearmodifiers','--delay','0','--',action['text']])
                                elif action['kind']=='move': result=run(['xdotool','mousemove',str(action['x']),str(action['y'])])
                                elif action['kind'] in ['click','scroll']:
                                    result=run(['xdotool','mousemove',str(action['x']),str(action['y'])]);assert result.returncode==0
                                    button=action['button'] if action['kind']=='click' else (4 if action['direction']=='up' else 5)
                                    count=action['count'] if action['kind']=='click' else action['steps']
                                    result=run(['xdotool','click','--repeat',str(count),'--delay','60',str(button)])
                                else: raise AssertionError('unsupported synthetic input')
                                assert result.returncode==0,result.stderr
                        elif operation=='capture_region':
                            assert window['active'] and body['expectedWindow']=={key:window[key] for key in ['x','y','width','height']}
                            region=body['region'];assert region['width']*region['height']<=1000000
                            captured=subprocess.run(['import','-window','root','-crop',f"{region['width']}x{region['height']}+{region['x']}+{region['y']}",'png:-'],env=env,capture_output=True,timeout=5)
                            assert captured.returncode==0 and captured.stdout.startswith(b'\x89PNG\r\n\x1a\n')
                            value=dict(image=dict(mimeType='image/png',data=base64.b64encode(captured.stdout).decode()),capture=dict(width=region['width'],height=region['height']),taskCompleted=False)
                        else: raise AssertionError('unsupported synthetic Native operation')
                        operations.append('native.'+operation)
                        if operation!='capture_region': value=dict(delivery='dispatched',taskCompleted=False)
                    self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(json.dumps(value).encode());return
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
                    value['windows'] = [{key:value for key,value in window.items() if key!='processIdentity'} for window in windows()]
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
        def fresh_requester(restart=False):
            nonlocal mcp
            mcp.terminate(); mcp.wait(timeout=10)
            if restart:
                manifest = json.loads((Path(env['AGENT_BROWSER_SOCKET_DIR']) / 'runtime-host.json').read_text())
                owner_pid = manifest['pid']
                owner = Path('/proc/' + str(owner_pid))
                owner_env = owner.joinpath('environ').read_bytes().split(bytes([0]))
                assert ('AGENT_BROWSER_HOME=' + env['AGENT_BROWSER_HOME']).encode() in owner_env
                assert b'AGENT_BROWSER_RUNTIME_HOST_PROCESS=1' in owner_env
                identity = process_identity(owner_pid)
                assert identity['executable'] == str(Path(args.binary).resolve())
                assert process_identity(owner_pid) == identity
                os.kill(owner_pid, signal.SIGTERM)
                deadline = time.monotonic() + 10
                while owner.exists() and owner.joinpath('cmdline').read_bytes():
                    assert time.monotonic() < deadline, 'exact fixture owner did not exit'
                    time.sleep(.02)
            with mcp_log.open('a') as diagnostics:
                mcp = spawn([str(Path(args.binary).resolve()), 'mcp', 'serve'], stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=diagnostics, text=True)
            rpc('initialize', dict(protocolVersion='2024-11-05', capabilities={}, clientInfo=dict(name='mode-recovery-fixture', version='1')))
            mcp.stdin.write(json.dumps(dict(jsonrpc='2.0', method='notifications/initialized')) + '\n');mcp.stdin.flush()
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
            catalog=rpc('tools/list',{})['tools']
            assert len(catalog)==16 and sum(len(json.dumps(tool['inputSchema'],separators=(',',':'))) for tool in catalog)<15000, 'bounded public MCP catalog'
            mcp.stdin.write(json.dumps(dict(jsonrpc='2.0', method='notifications/initialized')) + '\n')
            mcp.stdin.flush()
            if args.mode_conversion:
                request = dict(profile='manual-fixture',profileMode='persistent',mode='automation',taskKey='mode-roundtrip',
                    url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/login-fixture',serviceName='fixture',agentName='fixture',taskName='Mode roundtrip')
                task=call('browser_open',request)
                assert cookie_seeded.wait(10), 'seeded browser cookie'
                link=call('browser_handoff',dict(taskId=task['taskId']))
                original=(task['taskId'],task['browserId'],task['profileId'],link['handoffUrl'])
                prepared=call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-one',mode='cdp_free',choice='prepare')))
                assert prepared['phase']=='prepared',prepared
                original_pid = browsers()[0][0]
                if args.recovery_case == 'published-terminal':
                    journal = root / '.agent-browser/service/runtime.mode.sqlite3'
                    assert journal.is_file(), 'prepared operation journal required for fault injection'
                    with sqlite3.connect(journal) as connection:
                        connection.execute("CREATE TRIGGER fixture_terminal_loss BEFORE UPDATE OF terminal ON browser_mode_operations WHEN NEW.terminal=1 BEGIN SELECT RAISE(ABORT,'fixture_terminal_write_loss'); END")
                if args.recovery_case and args.recovery_case.startswith('pre-spawn'):
                    inject_pre_spawn.set()
                if args.recovery_case == 'native-restore':
                    suppress_replacement_windows.set()
                def convert(choice, confirmed=False):
                    return call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-one',mode='cdp_free',choice=choice,ownerConfirmed=confirmed)))
                if args.recovery_case == 'lost-reply':
                    seq += 1
                    mcp.stdin.write(json.dumps(dict(jsonrpc='2.0',id=seq,method='tools/call',params=dict(name='browser_command',arguments=dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-one',mode='cdp_free',choice='relaunch')))))+'\n');mcp.stdin.flush()
                    deadline=time.monotonic()+20
                    while not any(pid != original_pid for pid,_ in browsers()):
                        assert time.monotonic()<deadline,'replacement not launched';time.sleep(.02)
                    fresh_requester(args.restart_owner)
                    changed=convert('reconcile')
                else:
                    changed=convert('relaunch')
                if args.recovery_case == 'published-terminal':
                    assert changed['phase'] == 'launch_dispatched' and changed['failure'] == 'mode_operation_write_failed', changed
                    published_pid = browsers()[0][0]
                    assert published_pid != original_pid and len(browsers()) == 1
                    with sqlite3.connect(journal) as connection:
                        connection.execute('DROP TRIGGER fixture_terminal_loss')
                    if args.restart_owner: fresh_requester(True)
                    changed = convert('restore_previous', True)
                    assert changed['phase'] == 'completed' and changed['failure'] == 'mode_operation_already_completed', changed
                    assert changed['recoveryChoice'] is None, 'rejected restore must not persist an effect intent'
                    assert len(browsers()) == 1 and browsers()[0][0] == published_pid, 'published completion refusal must preserve its process'
                elif args.recovery_case and args.recovery_case != 'lost-reply':
                    wanted='replacement_pending' if args.recovery_case=='native-restore' else 'launch_absent'
                    deadline=time.monotonic()+15
                    while changed['phase'] != wanted:
                        assert time.monotonic()<deadline,changed
                        changed=convert('reconcile');time.sleep(.1)
                    assert changed['operationId']=='mode-one' and changed['failure'],changed
                    pending_processes=browsers()
                    assert len(pending_processes)==(1 if wanted=='replacement_pending' else 0),pending_processes
                    if args.restart_owner: fresh_requester(True)
                    Path(args.chrome).chmod(0o755)
                    suppress_replacement_windows.clear()
                    changed=convert('relaunch' if args.recovery_case=='pre-spawn-retry' else 'restore_previous',True)
                    deadline=time.monotonic()+15
                    while changed['phase']!='completed':
                        assert time.monotonic()<deadline,changed
                        changed=convert('reconcile');time.sleep(.1)
                    assert changed['operationId']=='mode-one' and changed['requestedMode']=='cdp_free',changed
                    assert len(browsers())==1,'recovery created concurrent processes'
                    if args.recovery_case!='pre-spawn-retry':
                        assert changed['effectiveMode']=='cdp' and changed['observedMode']=='cdp',changed
                        call('browser_navigate',dict(taskId=task['taskId'],url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/login-fixture'))
                        changed=call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-after-restore',mode='cdp_free',choice='relaunch')))
                for _ in range(30):
                    if changed['phase']=='completed': break
                    time.sleep(.1)
                    changed=call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-after-restore' if args.recovery_case in ['pre-spawn-restore','native-restore'] else 'mode-one',mode='cdp_free',choice='reconcile')))
                assert changed['phase']=='completed',changed
                physical=browsers();assert len(physical)==1 and '--remote-debugging' not in physical[0][1], 'no-CDP replacement proof'
                assert changed['taskId']==original[0] and changed['browserId']==original[1] and changed['profileId']==original[2]
                assert_conversion_executable_pin(original[2], 'cdp_free')
                cookie_after_close.set()
                call('browser_command',dict(taskId=task['taskId'],action='native_input',params=dict(actions=[dict(kind='key',chord='ctrl+l'),dict(kind='text',text=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/login-fixture',method='keys'),dict(kind='key',chord='Return')])))
                assert cookie_restored.wait(10),'native navigation independently delivers preserved cookie'
                captured=rpc('tools/call',dict(name='browser_command',arguments=dict(taskId=task['taskId'],action='native_capture',params={})))
                assert any(block.get('type')=='image' and block.get('mimeType')=='image/png' for block in captured.get('content',[])), 'native screenshot must be agent-visible image content'
                call('browser_command',dict(taskId=task['taskId'],action='native_input',params=dict(actions=[dict(kind='key',chord='ctrl+l'),dict(kind='text',text=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/native-login'),dict(kind='key',chord='Return')])))
                assert native_page.wait(5),'native login page served'
                time.sleep(.3)
                call('browser_command',dict(taskId=task['taskId'],action='native_input',params=dict(actions=[dict(kind='click',x=500,y=500,button=1,count=1),dict(kind='scroll',x=500,y=500,direction='down',steps=3)])))
                assert native_mouse.wait(5) and native_scroll.wait(5),'independently observed native click and scroll effects'
                call('browser_command',dict(taskId=task['taskId'],action='native_input',params=dict(actions=[dict(kind='key',chord='ctrl+Home'),dict(kind='key',chord='Tab'),dict(kind='text',text='synthetic-proof'),dict(kind='key',chord='Return')])))
                assert native_auth.wait(5),'native form login independently authenticated'
                back=call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-back',mode='cdp',choice='relaunch')))
                for _ in range(30):
                    if back['phase']=='completed': break
                    time.sleep(.1)
                    back=call('browser_command',dict(taskId=task['taskId'],action='mode_convert',params=dict(operationId='mode-back',mode='cdp',choice='reconcile')))
                assert back['phase']=='completed',back
                assert_conversion_executable_pin(original[2], 'cdp')
                observed=call('browser_command',dict(taskId=task['taskId'],action='mode'))
                assert observed['mode']=='cdp'
                final_link=call('browser_handoff',dict(taskId=task['taskId']))
                assert final_link['handoffUrl']==original[3], 'same durable handoff'
                call('browser_navigate',dict(taskId=task['taskId'],url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/login-fixture'))
                assert cookie_restored.is_set()
                native_auth.clear()
                call('browser_navigate',dict(taskId=task['taskId'],url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/auth-check'))
                assert native_auth.wait(5),'native login continuity after CDP return'
                call('browser_close',dict(taskId=task['taskId']))
                task=None
            else:
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
        if args.mode_conversion:
            print(json.dumps(dict(passed=True,scope='public MCP, real Chrome and synthetic Native/X11; actual installed Native not qualified',modeRoundTrip=True,persistentCookiePreserved=True,executablePinProvenance=executable_pin_provenance,cleanupVerified=True)))
        else:
            print(json.dumps(dict(passed=True, scope='public MCP, real Chrome/X11, synthetic r3 provider; no live Google or viewer',
                                  noCdp=True, headed=True, readyHandoff=True, replayPreserved=True, domRefused=True,
                                  closed=True, profilePreserved=True, persistentCookiePreserved=True,
                                  automationTransition=True, cleanupVerified=True)))



if __name__ == '__main__':
    main()
