"""Opt-in task-owned source executable startup, shared host and Unix transport probe."""
import json
import hashlib
import fcntl
import http.client
import os
from pathlib import Path
from urllib.parse import urlsplit
import secrets
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor


def run_runtime_host_fixture(spawn, env, root, artifacts, cli, chrome, workflow_log, ui_bundle=None, controller_fence=False):
    sockets = root / 'runtime-sockets'
    sockets.mkdir(mode=0o700)
    token = secrets.token_hex(32)
    auth = sockets / 'runtime-host.token'
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
    daemon = spawn([cli], stdout=workflow_log, stderr=workflow_log)
    env.pop('AGENT_BROWSER_RUNTIME_HOST_PROCESS')
    env.pop('AGENT_BROWSER_DAEMON_AUTH_TOKEN')
    manifest_path = sockets / 'runtime-host.json'
    deadline = time.monotonic() + 45
    while not manifest_path.exists():
        assert daemon.poll() is None, 'source runtime-host exited before publication'
        assert time.monotonic() < deadline, 'source runtime-host publication deadline'
        time.sleep(0.025)
    manifest = json.loads(manifest_path.read_text())
    assert manifest['pid'] == daemon.pid, 'runtime-host publication belongs to a different process'
    assert manifest['executableGeneration'] == hashlib.sha256(Path(cli).read_bytes()).hexdigest(), 'runtime-host executable generation differs from the reviewed pin'
    (artifacts / 'runtime-host-observation.json').write_text(json.dumps(manifest, indent=2) + '\n')

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
        assert json.loads(manifest_path.read_text())['pid'] == daemon.pid
        title = command(session, '--profile', str(profile), 'get', 'title')
        assert title['success'] and title['data']['title'] == session, 'exact session lost its synthetic document'
        command(session, '--profile', str(profile), 'tab', 'list', '--verbose')
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
        tab = next(tab for tab in state['tabs'].values() if tab['browserId'] == browser['id'])
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
