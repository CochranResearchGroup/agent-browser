#!/usr/bin/env python3
"""Public profile duplication with real Chrome/X11 and a synthetic Native owner."""
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
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--chrome', required=True)
    parser.add_argument('--delayed-close', action='store_true')
    parser.add_argument('--borrowed-source', action='store_true')
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
                '--namespace-worker', '--parent-proof-fd', str(read_fd), *(['--delayed-close'] if args.delayed_close else []), *(['--borrowed-source'] if args.borrowed_source else [])], pass_fds=(read_fd,)).returncode
        finally:
            os.close(read_fd)
        raise SystemExit(result)
    assert args.parent_proof_fd is not None, 'private namespace proof required'
    with os.fdopen(args.parent_proof_fd) as source:
        proof = json.load(source)
    assert proof['parentPid'] == os.getppid(), 'fixture parent changed'
    for key, previous in proof['namespaces'].items():
        assert os.readlink('/proc/self/ns/' + key) != previous, 'private namespace required: ' + key

    wire = json.loads((Path(__file__).resolve().parents[2] / 'docs/dev/contracts/remote-view-application-r3.v1.fixture.json').read_text())
    wire2=json.loads(json.dumps(wire).replace('assignment-external-1','assignment-external-2').replace('22222222-2222-4222-8222-222222222222','44444444-4444-4444-8444-444444444444').replace('33333333-3333-4333-8333-333333333333','55555555-5555-4555-8555-555555555555'))
    with tempfile.TemporaryDirectory(prefix='ab403-public-') as temporary:
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
        (root / 'hosts.json').write_text(json.dumps(dict(version=1, hosts={'fixture':{'origin':'https://browser.fixture.invalid','providerApplication':'agent-browser'}} if args.borrowed_source else {})))
        children, registered, operations = [], {}, []
        native_observed_pids=set()
        close_dispatches=[]
        displays=[]
        acquired_second=threading.Event()
        cookie_seeded = threading.Event()
        cookie_after_close = threading.Event()
        cookie_restored = threading.Event()
        get_requests = []
        def spawn(command, **kwargs):
            child = subprocess.Popen(command, env=kwargs.pop('env',env), start_new_session=True, **kwargs)
            children.append(child)
            return child
        def run(command,display=None):
            return subprocess.run(command, env=dict(env,DISPLAY=display or env.get('DISPLAY','')), capture_output=True, text=True, timeout=3)
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
            fields = Path('/proc/'+str(pid)+'/stat').read_text().rsplit(')',1)[1].split()
            return dict(pid=pid, start_token=fields[19], boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(), executable=os.readlink('/proc/'+str(pid)+'/exe'))
        def windows(display=None):
            if display is None:
                return [window for current in displays for window in windows(current)]
            result = []
            active = run(['xdotool', 'getactivewindow'],display).stdout.strip()
            for pid, _ in browsers():
                for window in run(['xdotool', 'search', '--onlyvisible', '--pid', str(pid)],display).stdout.split():
                    geometry = run(['xdotool', 'getwindowgeometry', '--shell', window],display)
                    values = dict(re.findall(r'^(X|Y|WIDTH|HEIGHT)=(-?\d+)$', geometry.stdout, re.M))
                    if len(values) == 4:
                        result.append(dict(id=int(window), title=run(['xdotool', 'getwindowname', window],display).stdout.strip(), **{'class': 'chrome'},
                                           pid=pid, x=int(values['X']), y=int(values['Y']),
                                           width=int(values['WIDTH']), height=int(values['HEIGHT']),
                                           active=window == active, maximized=False, processIdentity=process_identity(pid)))
            return result
        class Provider(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass
            def do_GET(self):
                if self.path not in ['/seed', '/check-copy', '/write-source', '/check-source', '/check-independent']:
                    self.send_response(404); self.end_headers(); return
                cookie = next((part.split('=',1)[1] for part in self.headers.get('Cookie','').split('; ') if part.startswith('ab403_session=')), '')
                get_requests.append(dict(path=self.path, cookie=cookie))
                self.send_response(200)
                if self.path == '/seed' and sum(record['path']=='/seed' for record in get_requests)==1: self.send_header('Set-Cookie','ab403_session=initial; Max-Age=3600; Path=/; HttpOnly; SameSite=Lax')
                if self.path == '/write-source': self.send_header('Set-Cookie','ab403_session=source-diverged; Max-Age=3600; Path=/; HttpOnly; SameSite=Lax')
                self.send_header('Content-Type','text/html'); self.end_headers()
                self.wfile.write(b'<!doctype html><title>Synthetic profile duplication</title>')
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                if self.path.startswith('/v1/agent-desktop/'):
                    operation=self.path.rsplit('/',1)[1]
                    selected=wire if body['desktopId']==wire['assignmentObservation']['target']['viewingDesktopId'] else wire2
                    assert body['desktopId']==selected['assignmentObservation']['target']['viewingDesktopId'] and body['expectedGeneration']==19
                    display=displays[0 if selected is wire else 1]
                    current=windows(display)
                    if operation=='observe':
                        native_observed_pids.update(window['pid'] for window in current)
                        value=dict(desktopId=body['desktopId'],generation=19,geometry=dict(current=dict(width=1280,height=800)),windows=dict(windows=current))
                    else:
                        window=next(w for w in current if w['id']==body['windowId'])
                        assert window['processIdentity']==body['expectedProcessIdentity'], 'Native process fence changed'
                        if operation=='window':
                            assert body['action']=='activate'; result=run(['xdotool','windowactivate','--sync',str(window['id'])],display); assert result.returncode==0,result.stderr
                        elif operation=='input':
                            assert window['active'], 'Native input target must be active'
                            assert body['expectedWidth']==1280 and body['expectedHeight']==800
                            assert body['expectedWindow']=={key:window[key] for key in ['x','y','width','height']}
                            for action in body['actions']:
                                if action['kind']=='key':
                                    if action['chord'].lower()=='alt+f4':
                                        close_dispatches.append(window['pid'])
                                        if args.delayed_close and len(close_dispatches)==1:
                                            # Failure injection pauses ONLY this exact synthetic Chrome.
                                            # Native WM close is genuinely dispatched while stopped; it
                                            # completes after the bounded continuation, not another close.
                                            identity=window['processIdentity'];os.kill(window['pid'],signal.SIGSTOP)
                                            def resume():
                                                try:
                                                    if process_identity(identity['pid'])==identity:os.kill(identity['pid'],signal.SIGCONT)
                                                except OSError:pass
                                            timer=threading.Timer(3.5,resume);timer.daemon=True;timer.start()
                                    result=run(['xdotool','key','--clearmodifiers',action['chord']],display)
                                elif action['kind']=='text': result=run(['xdotool','type','--clearmodifiers','--delay','0','--',action['text']],display)
                                else: raise AssertionError('unsupported synthetic input')
                                assert result.returncode==0,result.stderr
                        else: raise AssertionError('unsupported synthetic Native operation')
                        operations.append('native.'+operation)
                        value=dict(delivery='dispatched',taskCompleted=False)
                    self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(json.dumps(value).encode());return
                request = body['request']
                operation = request['operation']
                selected=wire2 if request.get('assignment_id')==wire2['assignment']['assignmentId'] else wire
                display=displays[0 if selected is wire else 1]
                operations.append(operation)
                value = None
                if operation == 'inventory':
                    value = copy.deepcopy(selected['inventory'])
                    value['policy']['pools']['main']['maximum_reserved'] = 2
                    if acquired_second.is_set():
                        value['assignments']+=copy.deepcopy(wire2['inventory']['assignments'])
                        value['desktops']+=copy.deepcopy(wire2['inventory']['desktops'])
                        value['pools'][0]['desktopMembers']+=copy.deepcopy(wire2['inventory']['pools'][0]['desktopMembers'])
                elif operation == 'acquire':
                    assert not acquired_second.is_set(),'fixture capacity exhausted';acquired_second.set();value=wire2['assignment']
                elif operation == 'activate':
                    result=run(['xdotool','windowactivate','--sync',str(request['window_id'])],display);assert result.returncode==0,result.stderr
                    value=copy.deepcopy(selected['activation']);value['windowId']=request['window_id']
                elif operation == 'observe_assignment':
                    value = selected['assignmentObservation']
                elif operation == 'launch_environment':
                    value = copy.deepcopy(selected['launchEnvironment'])
                    value['environment'] = {'DISPLAY': display, 'XAUTHORITY': str(root / 'Xauthority'),
                                            'REMOTE_VIEW_SLOT_GENERATION': str(selected['assignmentObservation']['target']['viewingGeneration'])}
                elif operation == 'windows':
                    value = copy.deepcopy(selected['windows'])
                    value['windows'] = [{key:value for key,value in window.items() if key!='processIdentity'} for window in windows(display)]
                elif operation == 'register_native_handoff_observation':
                    key = request['idempotencyKey']
                    application = body['application'].encode()
                    digest = hashlib.sha256(len(application).to_bytes(8, 'big') + application + key.encode()).hexdigest()
                    registered[key] = dict(correlationId=hashlib.sha256(key.encode()).hexdigest(),
                                           requestDigest=digest, application=body['application'],
                                           target=selected['assignmentObservation']['target'], targetDigest=request['targetDigest'],
                                           clock=dict(bootId='synthetic-boot', elapsedMs=100), lifetimeMs=request['lifetimeMs'], used=None)
                    value = dict(schemaVersion=1, observation=registered[key])
                elif operation == 'observe_native_handoff_use':
                    record = next(v for v in registered.values() if v['correlationId'] == request['correlationId'])
                    value = dict(schemaVersion=1, state='pending', observation=record)
                elif operation == 'read_idle_retirement':
                    value = dict(schemaVersion=1, target=selected['assignmentObservation']['target'],
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
            displays.append(env['DISPLAY'])
            read2,write2=os.pipe()
            try:
                with (root/'xvfb-second.log').open('w') as diagnostics:
                    second=spawn(['Xvfb','-displayfd',str(write2),'-screen','0','1280x800x24','-nolisten','tcp'],pass_fds=(write2,),stdout=subprocess.DEVNULL,stderr=diagnostics)
                os.close(write2);assert select.select([read2],[],[],10)[0],'second display readiness deadline'
                number=os.read(read2,64).decode().strip();assert number.isdecimal() and second.poll() is None
                displays.append(':'+number)
            finally:os.close(read2)
            spawn(['openbox','--sm-disable'],env=dict(env,DISPLAY=displays[1]),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            deadline=time.monotonic()+20
            while 'window id #' not in run(['xprop','-root','_NET_SUPPORTING_WM_CHECK'],displays[1]).stdout:
                assert time.monotonic()<deadline,'second window manager readiness deadline';time.sleep(.02)
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
            request = dict(profile='manual-fixture', profileMode='persistent', mode='manual_login', taskKey='source-work',
                           url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/seed', serviceName='fixture', agentName='fixture', taskName='Source work')
            if args.borrowed_source:
                with (root/'borrowed-chrome.log').open('w') as diagnostics:
                    borrowed=spawn([str(Path(args.chrome).resolve()),'--no-sandbox','--no-first-run','--no-default-browser-check','--disable-dev-shm-usage','--disable-sync','--remote-debugging-port=0','--user-data-dir='+str(profile),env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/seed'],stdout=subprocess.DEVNULL,stderr=diagnostics)
                deadline=time.monotonic()+10
                while not (profile/'DevToolsActivePort').exists():
                    assert borrowed.poll() is None and time.monotonic()<deadline,'borrowed Chrome readiness deadline';time.sleep(.02)
                port,path=(profile/'DevToolsActivePort').read_text().splitlines()[:2]
                pages=json.load(urllib.request.urlopen('http://127.0.0.1:'+port+'/json/list',timeout=3))
                page=next(page for page in pages if page['type']=='page')
                identity=process_identity(borrowed.pid)
                attachment=dict(version=1,process=dict(pid=borrowed.pid,startToken='linux:'+identity['boot_id']+':'+identity['start_token'],executablePath=identity['executable'],browserFamily='chrome'),profilePath=str(profile),cdpEndpoint='ws://127.0.0.1:'+port+path,targetId=page['id'])
                binding=dict(version=1,hostId='fixture',participantId='agent',sessionId='app-session',placement=dict(mode='required',desktop=dict(desktopId=wire['assignment']['desktopId'],generation=7)))
                attached_request={key:value for key,value in request.items() if key!='url'}
                attached_request.update(mode='automation',profileMode='existing',attachment=attachment,hostBinding=binding)
                task=call('browser_open',attached_request)
                # The supplying app owns reaping its original child on normal exit.
                def reap_app_original():
                    while borrowed.poll() is None:time.sleep(.02)
                threading.Thread(target=reap_app_original,daemon=True).start()
            else:task=call('browser_open',request)
            source_task=task
            assert task['operatorVisible']['state']=='ready', 'source Native handoff not ready'
            deadline=time.monotonic()+10
            while not any(record['path']=='/seed' for record in get_requests):
                assert time.monotonic()<deadline,'synthetic cookie seed navigation missing';time.sleep(.02)
            source_browser=browsers()[0][0]
            operation_id='synthetic-duplicate-one'
            def duplicate(choice='prepare',owner=False):
                return call('browser_command',dict(action='profile_duplicate',params=dict(operationId=operation_id,sourceProfileId=source_task['profileId'],destinationName='Independent fixture',choice=choice,ownerConfirmed=owner,launch=True)))
            prepared=duplicate()
            assert prepared['phase']=='awaiting_owner_choice',prepared
            assert source_task['taskId'] in prepared['affectedTaskIds'],prepared
            assert any(action['action']=='close_and_duplicate' and action['requiresConfirmation'] for action in prepared['actions']),prepared
            copied=duplicate('close_and_duplicate',True)
            deadline=time.monotonic()+15
            while copied['phase']!='completed':
                assert time.monotonic()<deadline,copied
                time.sleep(.1);copied=duplicate('reconcile')
            assert copied['copyComplete'] and copied['profilePublished'],copied
            assert len(close_dispatches)==1,'operation reconciliation replayed Native close'
            assert copied['source']['id']!=copied['destination']['id'],copied
            assert not Path('/proc/'+str(source_browser)).exists(),'original source process still exists'
            receipt=copied['launch'];assert receipt['taskId']!=source_task['taskId'] and receipt['browserId']!=source_task['browserId'],copied
            duplicate_task=receipt['taskId']
            assert len(browsers())==1,'copy should launch only destination; source must reopen only when selected'
            assert browsers()[0][0] in native_observed_pids,'completed launch lacks qualified Native window readiness'
            def native_visit(task_id,path):
                call('browser_command',dict(taskId=task_id,action='native_input',params=dict(actions=[dict(kind='key',chord='ctrl+l'),dict(kind='text',text=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+path),dict(kind='key',chord='Return')])))
                deadline=time.monotonic()+10
                while not any(record['path']==path for record in get_requests):
                    assert time.monotonic()<deadline,'native navigation missing: '+path;time.sleep(.02)
                return next(record['cookie'] for record in get_requests if record['path']==path)
            assert native_visit(duplicate_task,'/check-copy')=='initial','duplicate lost the synthetic authenticated cookie'
            source_reopened=call('browser_open',dict(request,taskKey='source-reopened',url=env['AGENT_BROWSER_REMOTE_VIEW_ORIGIN']+'/write-source'))
            assert len(browsers())==2,'source and independent duplicate must coexist'
            deadline=time.monotonic()+10
            while not any(record['path']=='/write-source' for record in get_requests):
                assert time.monotonic()<deadline,'source write navigation missing';time.sleep(.02)
            assert native_visit(source_reopened['taskId'],'/check-source')=='source-diverged','source write did not persist'
            # Native focus is explicit because the reopened source owns focus.
            destination_pid=next(pid for pid,_ in browsers() if pid!=next(pid for pid,argv in browsers() if '--user-data-dir='+str(profile) in argv))
            destination_window=next(window for window in windows() if window['pid']==destination_pid)
            assert run(['xdotool','windowactivate','--sync',str(destination_window['id'])]).returncode==0
            assert native_visit(duplicate_task,'/check-independent')=='initial','source mutation leaked into the copied profile'
            paths=[re.search(r'--user-data-dir=([^ ]+)',argv).group(1) for _,argv in browsers()]
            assert len(set(paths))==2,'two browsers shared a writable profile directory'
            assert len({os.stat(path).st_ino for path in paths})==2,'profile directories are not independent'
            replay=duplicate('reconcile');assert replay['launch']==receipt and len(browsers())==2,'operation replay launched another browser'
            call('browser_close',dict(taskId=source_reopened['taskId']));call('browser_close',dict(taskId=duplicate_task));task=None
            assert not browsers(),'closed synthetic browsers left owned process residue'

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
        print(json.dumps(dict(passed=True,scope='public MCP profile_duplicate, real Chrome/X11, synthetic Native owner',
            nativeGracefulClose=True,independentProfiles=True,concurrentBrowsers=True,sourcePreserved=True,
            syntheticCookieContinuity=True,divergentWrites=True,replayPreserved=True,cleanupVerified=True)))


if __name__ == '__main__':
    main()
