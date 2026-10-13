"""Opt-in task-owned source executable startup, shared host and Unix transport probe."""
import json
import hashlib
import fcntl
import http.client
import os
from pathlib import Path
from urllib.parse import urlsplit, urlencode
import secrets
import subprocess
import socket
import time
from concurrent.futures import ThreadPoolExecutor


def run_runtime_host_fixture(spawn, env, root, artifacts, cli, chrome, workflow_log, ui_bundle=None, controller_fence=False, native_eof=False, peer_read_only=False, cli_bootstrap=False, public_background=False, managed_session=False, managed_replay=False):
    sockets = root / 'runtime-sockets'
    sockets.mkdir(mode=0o700)
    token = secrets.token_hex(32)
    auth = sockets / 'runtime-host.token'
    if not cli_bootstrap:
        auth.write_text(token)
        auth.chmod(0o600)
    env.update({'AGENT_BROWSER_RUNTIME_HOST': '1',
                'AGENT_BROWSER_SOCKET_DIR': str(sockets),
                'AGENT_BROWSER_RUNTIME_HOST_PROCESS': '1',
                'AGENT_BROWSER_DAEMON_AUTH_TOKEN': token,
                'AGENT_BROWSER_SESSION': 'default',
                'AGENT_BROWSER_HEADED': '1',
                'AGENT_BROWSER_DEBUG': '1',
                'AGENT_BROWSER_DASHBOARD_AUTH_DIR': str(root / 'operator-auth'),
                'AGENT_BROWSER_EXECUTABLE_PATH': chrome,
                'AGENT_BROWSER_ARGS': '--no-sandbox,--no-proxy-server,--disable-background-networking',
                'AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY': 'disabled'})
    if managed_session:
        env['AGENT_BROWSER_SESSION_DISPLAY'] = env['DISPLAY']
    if cli_bootstrap:
        env.pop('AGENT_BROWSER_RUNTIME_HOST_PROCESS')
        env.pop('AGENT_BROWSER_DAEMON_AUTH_TOKEN')
        bootstraps = [spawn([cli,'--json','--session','default','stream','status'],stdout=-1,stderr=-1)
                      for _ in range(2)]
        deadline = time.monotonic() + 60
        owner = None
        while owner is None:
            candidates = []
            for bootstrap in bootstraps:
                children_path = Path(f'/proc/{bootstrap.pid}/task/{bootstrap.pid}/children')
                if children_path.exists():
                    candidates.extend((bootstrap.pid,child) for child in children_path.read_text().split())
            assert candidates or any(bootstrap.poll() is None for bootstrap in bootstraps), 'bootstrap CLIs ended before host ownership capture'
            for parent_pid,child in candidates:
                process = Path('/proc')/child
                try:
                    raw = (process/'stat').read_text();fields = raw[raw.rfind(')')+2:].split()
                    environment = (process/'environ').read_bytes().split(b'\0')
                    if (os.readlink(process/'exe') == cli
                        and f'HOME={root / "home"}'.encode() in environment
                        and f'AGENT_BROWSER_SOCKET_DIR={sockets}'.encode() in environment
                        and b'AGENT_BROWSER_RUNTIME_HOST_PROCESS=1' in environment
                        and int(fields[1]) == parent_pid and int(fields[3]) == int(child)):
                        owner = {'pid':int(child),'parentPid':parent_pid,'start':fields[19],
                                 'session':int(fields[3]),'executable':cli,'home':str(root/'home'),
                                 'socketDir':str(sockets)}
                        (root/'source-host-owned-session.json').write_text(json.dumps(owner,indent=2)+'\n')
                        break
                except FileNotFoundError:
                    continue
            assert owner or time.monotonic() < deadline, 'source host ownership capture timed out'
            if owner is None:
                time.sleep(0.025)
        bootstrap_replies = []
        for bootstrap in bootstraps:
            stdout,stderr = bootstrap.communicate(timeout=60)
            reply = json.loads(stdout)
            bootstrap_replies.append({'returncode':bootstrap.returncode,'reply':reply,'stderr':stderr.decode()})
            (artifacts/'source-cli-bootstrap.json').write_text(json.dumps({
                'clients':bootstrap_replies,'owner':owner},indent=2)+'\n')
            assert bootstrap.returncode == 0 and reply['success'], 'public source CLI bootstrap failed'
        assert bootstrap_replies[0]['reply']['data']['port'] == bootstrap_replies[1]['reply']['data']['port'], 'concurrent bootstrap returned different default lanes'
        assert len(list(sockets.glob('*.pid'))) == 1 and len(list(sockets.glob('*.sock'))) == 1, 'concurrent bootstrap published duplicate host endpoints'
        daemon_pid = owner['pid']
    else:
        daemon = spawn([cli], stdout=workflow_log, stderr=workflow_log)
        daemon_pid = daemon.pid
    env.pop('AGENT_BROWSER_RUNTIME_HOST_PROCESS',None)
    env.pop('AGENT_BROWSER_DAEMON_AUTH_TOKEN',None)
    manifest_path = sockets / 'runtime-host.json'
    deadline = time.monotonic() + 45
    while not manifest_path.exists():
        assert Path(f'/proc/{daemon_pid}').exists(), 'source runtime-host exited before publication'
        assert time.monotonic() < deadline, 'source runtime-host publication deadline'
        time.sleep(0.025)
    manifest = json.loads(manifest_path.read_text())
    assert manifest['pid'] == daemon_pid, 'runtime-host publication belongs to a different process'
    assert manifest['executableGeneration'] == hashlib.sha256(Path(cli).read_bytes()).hexdigest(), 'runtime-host executable generation differs from the reviewed pin'
    (artifacts / 'runtime-host-observation.json').write_text(json.dumps(manifest, indent=2) + '\n')
    if managed_session:
        from lib.p237_managed_session_fixture import run_managed_session_fixture
        run_managed_session_fixture(env, root, artifacts, sockets, manifest, token, workflow_log, managed_replay)
        return

    def command(session, *arguments):
        process = spawn([cli, '--json', '--session', session, *arguments],
                        stdout=-1, stderr=-1)
        stdout, stderr = process.communicate(timeout=35)
        response = json.loads(stdout) if stdout else {}
        retained_response = response
        if arguments == ('service', 'status') and response.get('success'):
            source_state = response['data']['service_state']
            retained_response = {'success': True, 'browserIds': sorted(source_state['browsers']),
                                 'tabIds': sorted(source_state['tabs'])}
        record = {'session': session, 'arguments': arguments,
                  'returncode': process.returncode,
                  'response': retained_response, 'stderr': stderr.decode()}
        workflow_log.write(json.dumps(record) + '\n')
        workflow_log.flush()
        assert process.returncode == 0, f'source CLI command failed: {record}'
        return response

    for session in ['p237-alpha', 'p237-beta']:
        profile = root / f'{session}-profile'
        command(session, '--headed', '--executable-path', chrome, '--profile', str(profile),
                'open', f'data:text/html,<title>{session}</title><p>{session}</p>')
        assert json.loads(manifest_path.read_text())['pid'] == daemon_pid
        title = command(session, '--profile', str(profile), 'get', 'title')
        assert title['success'] and title['data']['title'] == session, 'exact session lost its synthetic document'
        command(session, '--profile', str(profile), 'tab', 'list', '--verbose')
        if peer_read_only and session == 'p237-beta':
            command(session,'--profile',str(profile),'tab','new',
                    'data:text/html,<title>p237-beta-background</title><p>background</p>')
    status = command('p237-alpha', 'service', 'status')
    (artifacts / 'source-service-status.json').write_text(json.dumps(status, indent=2) + '\n')
    state = status['data']['service_state']
    def inventory(browser):
        endpoint = urlsplit(browser['cdpEndpoint'])
        connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=5)
        try:
            connection.request('GET', '/json/list')
            response = connection.getresponse()
            assert response.status == 200
            return sorted([{'id': tab['id'], 'title': tab['title'], 'url': tab['url']}
                           for tab in json.loads(response.read()) if tab['type'] == 'page'],
                          key=lambda tab: tab['id'])
        finally:
            connection.close()

    inventories = {browser_id: inventory(browser) for browser_id, browser in state['browsers'].items()}
    births = {}
    for browser in state['browsers'].values():
        raw = Path(f'/proc/{browser["pid"]}/stat').read_text()
        births[browser['id']] = {'pid': browser['pid'], 'start': raw[raw.rfind(')') + 2:].split()[19]}
    (artifacts / 'source-browser-births.json').write_text(json.dumps(births, indent=2) + '\n')
    # Only route/controller presentation setup is synthetic. Browser, target,
    # owner, parent policy and disconnected child grants come from the real CLI.
    state_path = root / 'home' / '.agent-browser' / 'service' / 'state.json'
    with (state_path.parent / 'state.json.lock').open('a') as lock:
        deadline = time.monotonic() + 3
        while True:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                assert time.monotonic() < deadline, 'task Service State setup lock deadline'
                time.sleep(0.025)
        persisted = json.loads(state_path.read_text())
        for session in ['p237-alpha', 'p237-beta']:
            browser_id = f'session:{session}'
            route_id = f'fixture-route:{session}'
            lease_id = f'fixture-controller:{session}'
            persisted['browsers'][browser_id]['displayAllocationId'] = 'fixture-display'
            persisted['remoteViewRoutes'][route_id] = {
                'id': route_id, 'browserId': browser_id, 'sessionId': session,
                'displayAllocationId': 'fixture-display', 'state': 'ready',
                'readOnly': False, 'controllerLeaseId': lease_id,
                'controllerEpoch': 1, 'viewerLeaseIds': [lease_id]}
            persisted['viewerLeases'][lease_id] = {
                'id': lease_id, 'routeId': route_id, 'browserId': browser_id,
                'viewerId': 'dashboard:admin', 'viewerRole': 'controller',
                'state': 'controlling', 'expiresAt': '2099-01-01T00:00:00Z'}
        persisted['stateRevision'] += 1
        temporary = state_path.parent / 'state.fixture.tmp'
        temporary.write_text(json.dumps(persisted))
        temporary.chmod(0o600)
        os.replace(temporary, state_path)
        fcntl.flock(lock, fcntl.LOCK_UN)
    port = int((sockets / 'default.stream').read_text())

    def source_http(path, body=None, cookie='', method='POST', timeout=15):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=timeout)
        try:
            connection.request(method, path, json.dumps(body) if body is not None else None,
                               {'Content-Type': 'application/json', 'Cookie': cookie})
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), json.loads(response.read())
        finally:
            connection.close()

    source_http('/api/dashboard-auth/status', method='GET')
    credentials = (root / 'operator-auth' / 'dashboard-auth.env').read_text()
    password = next(line.split('=', 1)[1].strip('"') for line in credentials.splitlines()
                    if line.startswith('AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD='))
    login_status, headers, _ = source_http('/api/dashboard-auth/login', {'username': 'admin', 'password': password})
    assert login_status == 200, 'task source HTTP synthetic login failed'
    cookie = headers['Set-Cookie'].split(';')[0]
    observations = []
    for session in ['p237-alpha', 'p237-beta', 'p237-alpha']:
        browser = state['browsers'][f'session:{session}']
        tab = next(tab for tab in state['tabs'].values()
                   if tab['browserId'] == browser['id'] and tab['title'] == session)
        focus_status, _, focus = source_http('/api/service/request', {
                            'action': 'view_focus', 'browserId': browser['id'],
                            'serviceName': 'P237Fixture', 'agentName': 'admin',
                            'taskName': 'synthetic-native-focus',
                            'jobTimeoutMs': 5000,
                            'params': {'targetId': tab['targetId'], 'maximize': True,
                                       'sessionName': session, 'operatorFocus': True,
                                       'routeId': f'fixture-route:{session}',
                                       'controllerLeaseId': f'fixture-controller:{session}'}}, cookie)
        (artifacts / 'source-focus-response.json').write_text(json.dumps({'status':focus_status,'reply':focus}, indent=2)+'\n')
        assert focus_status == 200 and focus['success'], f'actual source focus rejected: {focus_status}, {focus}'
        active = subprocess.check_output(['xdotool', 'getactivewindow'], env=env, text=True).strip()
        title = subprocess.check_output(['xdotool', 'getwindowname', active], env=env, text=True).strip()
        properties = subprocess.check_output(['xprop', '-id', active, '_NET_WM_PID', '_NET_WM_STATE'], env=env, text=True)
        input_window = subprocess.check_output(['xdotool', 'getwindowfocus'], env=env, text=True).strip()
        stacking = subprocess.check_output(['xprop', '-root', '_NET_CLIENT_LIST_STACKING'], env=env, text=True).strip()
        topmost = int(stacking.rsplit(',', 1)[-1].split()[-1], 16) == int(active)
        observation = {'session': session, 'activeWindow': active, 'inputWindow': input_window,
                       'title': title, 'properties': properties, 'topmost': topmost, 'reply': focus}
        observations.append(observation)
        (artifacts / 'source-native-focus.json').write_text(json.dumps(observations, indent=2) + '\n')
        assert session in title and active == input_window and topmost
        assert f'_NET_WM_PID(CARDINAL) = {browser["pid"]}' in properties.splitlines()
        assert '_NET_WM_STATE_MAXIMIZED_HORZ' in properties and '_NET_WM_STATE_MAXIMIZED_VERT' in properties
        peer = 'p237-beta' if session == 'p237-alpha' else 'p237-alpha'
        assert inventory(state['browsers'][f'session:{peer}']) == inventories[f'session:{peer}'], 'native selection changed the peer targets or documents'
        assert json.loads(manifest_path.read_text()) == manifest, 'source host identity changed during selection'
    if ui_bundle:
        identities = []
        for session in ['p237-alpha', 'p237-beta']:
            browser = state['browsers'][f'session:{session}']
            tab = next(tab for tab in state['tabs'].values() if tab['browserId'] == browser['id'])
            identities.append({'browserId':browser['id'], 'sessionName':session,
                               'routeId':f'fixture-route:{session}',
                               'controllerLeaseId':f'fixture-controller:{session}',
                               'displayAllocationId':'fixture-display',
                               'port':browser['viewStreams'][0]['readiness']['streamPort'],
                               'targetId':tab['targetId'], 'title':tab['title']})
        config_path = root / 'ui-selection-config.json'
        config_path.write_text(json.dumps({**identities[0], 'peer':identities[1],
                                          'tabs':[[item['targetId'], item['title']] for item in identities],
                                          'origin':f'http://127.0.0.1:{port}', 'cookie':cookie,
                                          'chrome':chrome, 'root':str(root), 'bundle':ui_bundle}))
        config_path.chmod(0o600)
        runner = Path(__file__).resolve().parent.parent / 'test-p237-workspace-selection.cjs'
        node = spawn(['node', str(runner), str(config_path)], stdout=workflow_log, stderr=workflow_log)
        assert node.wait(timeout=30) == 0, 'actual viewport source transport failed'
        active = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
        title = subprocess.check_output(['xdotool','getwindowname',active],env=env,text=True).strip()
        input_window = subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True).strip()
        properties = subprocess.check_output(['xprop','-id',active,'_NET_WM_PID','_NET_WM_STATE'],env=env,text=True)
        stacking = subprocess.check_output(['xprop','-root','_NET_CLIENT_LIST_STACKING'],env=env,text=True).strip()
        topmost = int(stacking.rsplit(',',1)[-1].split()[-1],16) == int(active)
        final = {'expected':'p237-beta','activeWindow':active,'inputWindow':input_window,
                 'title':title,'properties':properties,'topmost':topmost}
        (artifacts/'source-ui-final.json').write_text(json.dumps(final,indent=2)+'\n')
        assert 'p237-beta' in title, f'latest cross-browser UI selection lost: {title}'
        assert active == input_window and topmost
        assert f'_NET_WM_PID(CARDINAL) = {births["session:p237-beta"]["pid"]}' in properties.splitlines()
        assert '_NET_WM_STATE_MAXIMIZED_HORZ' in properties and '_NET_WM_STATE_MAXIMIZED_VERT' in properties
        assert all(inventory(browser) == inventories[browser_id] for browser_id,browser in state['browsers'].items()), 'UI selection changed source documents or targets'
    if public_background:
        target_url = 'data:text/html,<title>p237-public-background</title><p>public target</p>'
        profile_id = state['browsers']['session:p237-beta']['profileId']
        plan_status, _, plan = source_http('/api/service/access-plan?' + urlencode({
            'runtimeProfile':profile_id, 'sessionName':'p237-beta',
            'serviceName':'agent-browser-cli', 'agentName':'p237-beta',
            'url':target_url}), cookie=cookie, method='GET')
        (artifacts/'source-public-background-plan.json').write_text(json.dumps({'status':plan_status,'reply':plan},indent=2)+'\n')
        assert plan_status == 200 and plan['success'], 'public access plan failed'
        planned = plan['data']['decision']['serviceRequest']
        assert planned['available'] and planned['request']['browserId'] == 'session:p237-beta'
        assert planned['request']['sessionName'] == 'p237-beta' and planned['request']['runtimeProfile'] == profile_id
        def public_target_request(action, params):
            status, _, reply = source_http('/api/service/request', {
                'action':action, 'browserId':'session:p237-beta',
                'serviceName':'agent-browser-cli', 'agentName':'p237-beta',
                'taskName':'synthetic-public-background', 'jobTimeoutMs':10000,
                'params':{'sessionName':'p237-beta', 'runtimeProfile':profile_id, **params}}, cookie, timeout=20)
            return {'status':status, 'reply':reply}
        created = public_target_request('tab_new', {'url':target_url})
        (artifacts/'source-public-background-create.json').write_text(json.dumps(created,indent=2)+'\n')
        assert created['status'] == 200 and created['reply']['success'], 'public target creation failed'
        current = command('p237-beta','service','status')['data']['service_state']
        target = next(tab for tab in current['tabs'].values() if tab.get('url') == target_url)
        (artifacts/'source-public-background-target.json').write_text(json.dumps(target,indent=2)+'\n')
        assert target.get('profileAccess') is not None, 'public creator did not establish child access'
        # Restore Alpha through the same authenticated operator path before
        # observing the exact newly authorized Beta target.
        alpha = next(tab for tab in state['tabs'].values() if tab.get('title') == 'p237-alpha')
        status, _, selected = source_http('/api/service/request', {
            'action':'view_focus','browserId':'session:p237-alpha',
            'serviceName':'P237Fixture','agentName':'admin','taskName':'synthetic-native-focus',
            'params':{'sessionName':'p237-alpha','targetId':alpha['targetId'],
                      'operatorFocus':True,'maximize':True,
                      'routeId':'fixture-route:p237-alpha',
                      'controllerLeaseId':'fixture-controller:p237-alpha'}},cookie)
        assert status == 200 and selected['success'], 'public Alpha selection failed'
        def public_foreground():
            active = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
            return {'activeWindow':active,
                    'inputWindow':subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True).strip(),
                    'title':subprocess.check_output(['xdotool','getwindowname',active],env=env,text=True).strip(),
                    'properties':subprocess.check_output(['xprop','-id',active,'_NET_WM_PID','_NET_WM_STATE'],env=env,text=True)}
        before = public_foreground()
        assert 'p237-alpha' in before['title'] and before['activeWindow'] == before['inputWindow']
        assert f'_NET_WM_PID(CARDINAL) = {births["session:p237-alpha"]["pid"]}' in before['properties'].splitlines()
        preserved = {browser_id:inventory(browser) for browser_id,browser in state['browsers'].items()}
        (artifacts/'source-public-background-before.json').write_text(json.dumps({'foreground':before,'inventories':preserved},indent=2)+'\n')
        reads = []
        for action,field,expected in [('title','title','p237-public-background'),('url','url',target_url)]:
            result = public_target_request(action, {'targetId':target['targetId']})
            foreground = public_foreground()
            reads.append({'action':action, **result, 'foreground':foreground})
            (artifacts/'source-public-background-reads.json').write_text(json.dumps(reads,indent=2)+'\n')
            assert result['status'] == 200 and result['reply']['success'], 'authorized public target read failed'
            assert result['reply']['data'][field] == expected, 'public observation returned a different target'
            assert foreground == before, 'public background observation changed physical foreground'
            assert all(inventory(browser) == preserved[browser_id] for browser_id,browser in state['browsers'].items()), 'public observation changed documents or targets'
        assert inventory(state['browsers']['session:p237-alpha']) == inventories['session:p237-alpha'], 'public observation changed peer documents'
    if peer_read_only:
        before_window = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
        before_title = subprocess.check_output(['xdotool','getwindowname',before_window],env=env,text=True).strip()
        reply = command('p237-beta','--profile',str(root/'p237-beta-profile'),'get','title')
        after_window = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
        after_title = subprocess.check_output(['xdotool','getwindowname',after_window],env=env,text=True).strip()
        (artifacts/'source-peer-read.json').write_text(json.dumps({
            'beforeWindow':before_window,'beforeTitle':before_title,
            'afterWindow':after_window,'afterTitle':after_title,'reply':reply},indent=2)+'\n')
        assert reply['success'] and reply['data']['title'] == 'p237-beta'
        assert before_window == after_window and before_title == after_title, 'read-only peer title changed native focus'
        background = next(tab for tab in state['tabs'].values() if 'p237-beta-background' in (tab.get('url') or ''))
        assert background['profileAccess'] is None, 'background admission diagnostic requires the observed missing child grant'
        socket_path = next(path for path in sockets.iterdir() if path.is_socket()
                           and f'unix:{path.stat().st_dev}:{path.stat().st_ino}' == manifest['socketIdentity'])
        targeted_reads = []
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as transport:
            transport.settimeout(35)
            transport.connect(str(socket_path))
            with transport.makefile('rb') as reader:
                for action,field in [('title','title'),('url','url')]:
                    request = {'id':f'p237-peer-{action}','action':action,
                        'targetId':background['targetId'],'_agentBrowserAuthToken':token,
                        '_agentBrowserRuntimeLane':'p237-beta','profile':str(root/'p237-beta-profile')}
                    transport.sendall((json.dumps(request)+'\n').encode())
                    reply = json.loads(reader.readline())
                    active = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
                    targeted_reads.append({'action':action,'targetId':background['targetId'],
                                           'reply':reply,'activeWindow':active})
                    (artifacts/'source-peer-targeted-reads.json').write_text(json.dumps(targeted_reads,indent=2)+'\n')
                    assert not reply['success'] and 'current child access record is missing' in reply['error'], 'ungranted background target bypassed child admission'
                    assert active == before_window, 'background target read changed native foreground'
        assert all(inventory(browser) == inventories[browser_id] for browser_id,browser in state['browsers'].items())
    if native_eof:
        def eof_foreground():
            active = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
            return {'activeWindow':active,
                    'inputWindow':subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True).strip(),
                    'title':subprocess.check_output(['xdotool','getwindowname',active],env=env,text=True).strip(),
                    'properties':subprocess.check_output(['xprop','-id',active,'_NET_WM_PID','_NET_WM_STATE'],env=env,text=True)}
        foreground_boundaries = {'before':eof_foreground()}
        def eof_boundary(name):
            foreground_boundaries[name] = eof_foreground()
            (artifacts/'source-eof-foreground.json').write_text(json.dumps(foreground_boundaries,indent=2)+'\n')
        foreground_before_eof = {
            name:subprocess.check_output(['xdotool',command],env=env,text=True).strip()
            for name,command in [('activeWindow','getactivewindow'),('inputWindow','getwindowfocus')]}
        socket_path = next(path for path in sockets.iterdir() if path.is_socket()
                           and f'unix:{path.stat().st_dev}:{path.stat().st_ino}' == manifest['socketIdentity'])
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as transport:
            transport.settimeout(10)
            transport.connect(str(socket_path))
            def native_send(request):
                transport.sendall((json.dumps({**request, '_agentBrowserAuthToken':token,
                    '_agentBrowserRuntimeLane':'p237-alpha',
                    'profile':str(root / 'p237-alpha-profile')}) + '\n').encode())
            native_send({'id':'p237-eof-title','action':'title'})
            received = b''
            while b'\n' not in received:
                chunk = transport.recv(65536)
                assert chunk, 'source native title transport ended without a reply'
                received += chunk
            title_reply = json.loads(received.split(b'\n',1)[0])
            (artifacts/'source-eof-title.json').write_text(json.dumps(title_reply,indent=2)+'\n')
            assert title_reply['success'] and title_reply['data']['title'] == 'p237-alpha'
            connection_id = title_reply['terminalOutcome']['provenance']['connectionInstanceId']
            active_state = json.loads(state_path.read_text())
            owned_tabs = [tab for tab in active_state['tabs'].values()
                          if tab.get('profileAccess',{}).get('connectionInstanceId') == connection_id]
            assert owned_tabs and all(tab['profileAccess']['connectionState'] == 'active' for tab in owned_tabs)
            browser = state['browsers']['session:p237-alpha']
            endpoint = urlsplit(browser['cdpEndpoint'])
            connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=5)
            try:
                connection.request('GET','/json/list')
                response = connection.getresponse()
                assert response.status == 200
                page_socket = next(tab['webSocketDebuggerUrl'] for tab in json.loads(response.read())
                                   if tab['id'] == owned_tabs[0]['targetId'])
            finally:
                connection.close()
            native_send({'id':'p237-eof-wait','action':'wait','timeout':7000,'jobTimeoutMs':9000,
                'function':'(() => { window.__p237EOFStarted ??= Date.now(); return Date.now() - window.__p237EOFStarted >= 4000; })()'})
            deadline = time.monotonic() + 3
            while True:
                reader = spawn(['node',str(Path(__file__).parent/'p237_cdp_value.cjs'),page_socket,
                                '({startedAtMs:window.__p237EOFStarted, observedAtMs:Date.now(), stillWaiting:Boolean(window.__p237EOFStarted) && Date.now()-window.__p237EOFStarted<4000})'],stdout=-1,stderr=-1)
                marker, errors = reader.communicate(timeout=6)
                assert reader.returncode == 0, errors.decode()
                running_state = json.loads(state_path.read_text())
                wait_job = running_state['jobs'].get('p237-eof-wait')
                predicate_readback = json.loads(marker)
                if predicate_readback['stillWaiting'] and wait_job and wait_job['state'] == 'running':
                    break
                assert time.monotonic() < deadline, 'native wait did not enter its real Chrome predicate'
                time.sleep(0.025)
            eof_boundary('predicateObserved')
            transport.shutdown(socket.SHUT_RDWR)
        deadline = time.monotonic() + 35
        disconnected_while_running = False
        peer_while_running = None
        while True:
            after_eof = json.loads(state_path.read_text())
            grants = [after_eof['tabs'][tab['id']]['profileAccess'] for tab in owned_tabs]
            disconnected = all(grant['connectionInstanceId'] == connection_id
                               and grant['connectionState'] == 'disconnected' for grant in grants)
            wait_job = after_eof['jobs']['p237-eof-wait']
            (artifacts/'source-eof-progress.json').write_text(json.dumps({
                'grants':grants,'job':wait_job,
                'disconnectedWhileRunning':disconnected_while_running},indent=2)+'\n')
            if disconnected and wait_job['state'] == 'running':
                disconnected_while_running = True
                if peer_while_running is None:
                    eof_boundary('disconnectedBeforePeer')
                    peer_while_running = command('p237-beta','--profile',str(root/'p237-beta-profile'),'get','title')
                    eof_boundary('afterPeerTitle')
                    assert peer_while_running['data']['title'] == 'p237-beta'
                    assert json.loads(state_path.read_text())['jobs']['p237-eof-wait']['state'] == 'running', 'peer command only completed after Alpha work ended'
            if wait_job['state'] in ['succeeded', 'failed', 'cancelled', 'timed_out']:
                break
            assert time.monotonic() < deadline, 'native EOF did not release ownership and complete the submitted wait'
            time.sleep(0.025)
        assert disconnected_while_running, 'disconnect was only observed after running work finished'
        assert peer_while_running is not None, 'peer lane was not exercised during Alpha work'
        assert disconnected and wait_job['state'] == 'succeeded', 'native EOF did not preserve disconnection and successful harmless wait'
        assert wait_job['terminalOutcome']['provenance']['connectionInstanceId'] == connection_id
        assert any(event.get('terminalOutcome') == wait_job['terminalOutcome']
                   for event in after_eof['events']), 'EOF job terminal outcome absent from durable Service events'
        assert all(inventory(browser) == inventories[browser_id] for browser_id,browser in state['browsers'].items())
        assert json.loads(manifest_path.read_text()) == manifest
        foreground_after_eof = {
            name:subprocess.check_output(['xdotool',command],env=env,text=True).strip()
            for name,command in [('activeWindow','getactivewindow'),('inputWindow','getwindowfocus')]}
        eof_boundary('terminal')
        assert foreground_after_eof == foreground_before_eof, 'EOF or peer read changed physical focus'
        resumed = command('p237-alpha', '--profile', str(root/'p237-alpha-profile'), 'get', 'title')
        assert resumed['data']['title'] == 'p237-alpha'
        (artifacts/'source-eof-observation.json').write_text(json.dumps({
            'connectionInstanceId':connection_id,'tabIds':[tab['id'] for tab in owned_tabs],
            'predicateObservedInChrome':True,
            'predicateReadback':predicate_readback, 'peerReplyWhileRunning':peer_while_running,
            'foregroundBefore':foreground_before_eof,'foregroundAfter':foreground_after_eof,
            'disconnectedWhileRunning':disconnected_while_running,'terminalJob':wait_job,
            'sameHostContinued':True},indent=2)+'\n')
    if controller_fence:
        route_id = 'fixture-route:p237-alpha'
        before = json.loads(state_path.read_text())
        before_route = before['remoteViewRoutes'][route_id]
        before_profiles = before['profiles']
        active_before = subprocess.check_output(['xdotool', 'getactivewindow'], env=env, text=True).strip()
        identity = '\0'.join([env.get('AGENT_BROWSER_RUNTIME_ENVIRONMENT', 'production'), route_id, 'fixture-display'])
        fence_path = state_path.parent / 'desktop-input' / 'fences' / (hashlib.sha256(identity.encode()).hexdigest() + '.lock')
        # This is the existing product route fence, scoped to the disposable home.
        # No controller epoch is manufactured and no product callback is replaced.
        with fence_path.open('r+') as fence, ThreadPoolExecutor(max_workers=1) as executor:
            fcntl.flock(fence, fcntl.LOCK_EX | fcntl.LOCK_NB)
            try:
                takeover_request = {
                    'action': 'service_controller_lease_takeover',
                    'browserId': 'session:p237-alpha', 'serviceName': 'P237Fixture',
                    'agentName': 'admin', 'taskName': 'synthetic-controller-fence',
                    'jobTimeoutMs': 10000,
                    'params': {'routeId': route_id, 'sessionName': 'p237-alpha',
                               'viewerLeaseId': 'fixture-controller:replacement'}}
                pending = executor.submit(source_http, '/api/service/request', takeover_request, cookie, timeout=35)
                deadline = time.monotonic() + 3
                while True:
                    current = json.loads(state_path.read_text())
                    jobs = [job for job in current['jobs'].values()
                            if job.get('action') == 'service_controller_lease_takeover'
                            and job.get('state') == 'running']
                    if jobs:
                        break
                    assert not pending.done(), 'public takeover completed before observed fence contention'
                    assert time.monotonic() < deadline, 'public takeover did not reach running admission'
                    time.sleep(0.025)
                contention_started = time.monotonic()
                contention_status, _, contention = pending.result(timeout=35)
                contention_elapsed = time.monotonic() - contention_started
                (artifacts / 'source-controller-contention-response.json').write_text(json.dumps({
                    'status': contention_status, 'reply': contention,
                    'observationSeconds': contention_elapsed}, indent=2) + '\n')
                blocked = json.loads(state_path.read_text())
                assert not contention.get('success'), 'public takeover ignored the held external fence'
                assert 'desktop_input_route_fence_contended' in json.dumps(contention), f'public takeover failed outside the actual fence: {contention_status}, {contention}'
                assert blocked['remoteViewRoutes'][route_id]['controllerEpoch'] == before_route['controllerEpoch']
                assert blocked['remoteViewRoutes'][route_id]['controllerLeaseId'] == before_route['controllerLeaseId']
                assert blocked['profiles'] == before_profiles
                assert subprocess.check_output(['xdotool', 'getactivewindow'], env=env, text=True).strip() == active_before
                assert all(inventory(browser) == inventories[browser_id] for browser_id, browser in state['browsers'].items())
                assert contention['failure']['code'] == 'desktop_input_route_fence_contended', 'public contention lost its actionable failure code'
                assert contention['failure']['effectState'] == 'no_effect', 'pre-authority contention incorrectly advertises uncertain effects'
                assert contention['failure']['phase'] == 'file_lock_wait'
                assert contention['failure']['retryDisposition'] == 'inspect_before_retry'
                assert contention['terminalOutcome']['effectState'] == 'no_effect'
                terminal_job = blocked['jobs'][contention['id']]
                assert terminal_job['failure'] == contention['failure']
                assert terminal_job['terminalOutcome'] == contention['terminalOutcome']
                assert any(event.get('terminalOutcome') == contention['terminalOutcome']
                           for event in blocked['events']), 'public contention outcome absent from durable Service events'
            finally:
                fcntl.flock(fence, fcntl.LOCK_UN)
            # The failure says inspect_before_retry. Inspection above proves
            # unchanged authority, policy and native targets; release removes
            # the exact contention cause before this new explicit operation.
            takeover_status, _, takeover = source_http('/api/service/request', takeover_request, cookie)
        assert takeover_status == 200 and takeover['success'], f'public takeover rejected after release: {takeover_status}, {takeover}'
        after = json.loads(state_path.read_text())
        after_route = after['remoteViewRoutes'][route_id]
        assert after_route['controllerEpoch'] == before_route['controllerEpoch'] + 1
        assert after_route['controllerLeaseId'] == 'fixture-controller:replacement'
        assert after['profiles'] == before_profiles, 'controller takeover changed profile policy or identity'
        assert subprocess.check_output(['xdotool', 'getactivewindow'], env=env, text=True).strip() == active_before
        assert all(inventory(browser) == inventories[browser_id] for browser_id, browser in state['browsers'].items())
        (artifacts / 'source-controller-fence.json').write_text(json.dumps({
            'runningJobIds': [job['id'] for job in jobs],
            'contentionStatus': contention_status, 'contentionReply': contention,
            'blockedEpoch': blocked['remoteViewRoutes'][route_id]['controllerEpoch'],
            'releasedEpoch': after_route['controllerEpoch'], 'reply': takeover,
            'scope': 'real public takeover blocked by task-held external fence; no concurrent native effect'}, indent=2) + '\n')
    for session in ['p237-alpha', 'p237-beta']:
        command(session, '--profile', str(root / f'{session}-profile'), 'close')
        birth = births[f'session:{session}']
        deadline = time.monotonic() + 5
        while Path(f'/proc/{birth["pid"]}/stat').exists():
            try:
                raw = Path(f'/proc/{birth["pid"]}/stat').read_text()
            except FileNotFoundError:
                break
            if raw[raw.rfind(')') + 2:].split()[19] != birth['start']:
                break
            assert time.monotonic() < deadline, 'source close left its exact browser process'
            time.sleep(0.025)
    (artifacts / 'source-browser-close.json').write_text(json.dumps({'exactBrowsersExited': births}, indent=2) + '\n')
    workflow_log.write('runtime-host-workflow: passed\n')
    workflow_log.flush()
