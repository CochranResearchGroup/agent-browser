"""Public synthetic dashboard login and focus against a real managed target.

Only the provider route/controller fixture is synthetic. Browser/session/target
identity must be supplied by the actual managed owner, never seeded as legacy
Service State browser records.
"""
import fcntl
import http.client
import json
import os
from pathlib import Path


def run_managed_operator_fixture(root, artifacts, sockets, browser, pages, foreground, targets, expected_targets, spawn, source_cli, environment):
    browser_id = browser["id"]
    desktop_id = browser["desktop"]["desktop_id"]
    port = int((sockets / 'default.stream').read_text())

    def exchange(path, body=None, cookie='', method='POST'):
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=15)
        try:
            connection.request(method, path, json.dumps(body) if body is not None else None,
                               {'Content-Type': 'application/json', 'Cookie': cookie})
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), json.loads(response.read())
        finally:
            connection.close()

    status, _, payload = exchange('/api/dashboard-auth/status', method='GET')
    assert status == 200, 'synthetic dashboard auth status failed'
    credentials = (root / 'operator-auth' / 'dashboard-auth.env').read_text()
    password = next(line.split('=', 1)[1].strip('"') for line in credentials.splitlines()
                    if line.startswith('AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD='))
    status, headers, payload = exchange('/api/dashboard-auth/login', {'username': 'admin', 'password': password})
    assert status == 200 and payload.get('authenticated'), 'synthetic managed operator login failed'
    cookie = headers['Set-Cookie'].split(';')[0]
    status, _, payload = exchange('/api/service/status', cookie=cookie, method='GET')
    assert status == 200, f'managed public inventory failed: {payload}'
    (artifacts / 'managed-public-service-status.json').write_text(json.dumps(payload, indent=2) + '\n')
    workspace = payload.get('data', {}).get('browserRuntime', {}).get('workspace')
    assert workspace and browser_id in workspace.get('browsers', {}), 'public dashboard inventory omits its real managed browser'
    expected_ids = {page['browserSession']['targetId'] for _, page in pages}
    assert expected_ids <= {tab['targetId'] for tab in workspace['tabs'].values()}, 'managed public inventory lost task tabs'
    assert all('cdpEndpoint' not in item for item in workspace['browsers'].values()), 'dashboard inventory leaked managed debugger endpoints'
    # Native Remote View does not create legacy Agent Browser viewer leases.
    # A real managed selection must also work before the legacy compatibility
    # controller fixture below exists, through native assignment authority.
    native_lane, native_page = pages[-1]
    native_status, _, native_reply = exchange('/api/service/request', {
        'action': 'view_focus', 'browserId': browser_id, 'sessionName': native_lane,
        'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
        'taskName': 'Select native managed task without legacy viewer grants',
        'params': {'targetId': native_page['browserSession']['targetId'],
                   'sessionName': native_lane, 'operatorFocus': True, 'maximize': True},
    }, cookie=cookie)
    native_scene = foreground()
    (artifacts / 'managed-native-assignment-focus.json').write_text(json.dumps({'status': native_status, 'reply': native_reply, 'scene': native_scene}, indent=2) + '\n')
    assert native_status == 200 and native_reply.get('success'), f'native managed selection required a legacy viewer grant: {native_reply}'
    assert expected_targets[native_page['browserSession']['targetId']] in native_scene['title']
    managed_queued_authority_probe(spawn, browser, pages, exchange, cookie, artifacts, foreground, targets, None, native=True)
    from urllib.parse import urlsplit
    endpoint = urlsplit(browser['cdpEndpoint'])
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=5)
    try:
        connection.request('GET', '/json/list')
        response = connection.getresponse()
        owned_page = next(item for item in json.loads(response.read()) if item['id'] == native_page['browserSession']['targetId'])
    finally:
        connection.close()
    def draft_value(expression):
        helper = Path(__file__).resolve().parent / 'p237_cdp_value.cjs'
        process = spawn(['node', str(helper), owned_page['webSocketDebuggerUrl'], expression], stdout=-1, stderr=-1)
        stdout, stderr = process.communicate(timeout=8)
        assert process.returncode == 0, f'owned draft readback failed: {stderr.decode()}'
        return json.loads(stdout)
    assert draft_value("globalThis.__p237Draft = 'kept across sharing'") == 'kept across sharing'
    shared_status, _, shared = exchange('/api/service/request', {
        'action': 'remote_view_open', 'browserId': browser_id, 'sessionName': native_lane,
        'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
        'taskName': 'Open existing native target without navigation',
        'params': {'targetId': native_page['browserSession']['targetId'],
                   'sessionName': native_lane, 'displayAllocationId': desktop_id},
    }, cookie=cookie)
    (artifacts / 'managed-existing-target-handoff.json').write_text(json.dumps({'status': shared_status, 'reply': shared}, indent=2) + '\n')
    assert shared_status == 200 and shared.get('success'), f'existing native target handoff required navigation: {shared}'
    assert shared['data']['operatorVisible']['state'] == 'ready'
    assert shared['data']['handoffUrl'].startswith('/remote-view/'), 'native handoff did not use the durable entry point'
    assert draft_value('globalThis.__p237Draft') == 'kept across sharing', 'sharing reloaded the live task and lost in-memory work'
    # The dashboard backend owns native presentation routing. A stream port
    # serves assets and service APIs but is not the operator ingress boundary.
    import socket
    import time
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        dashboard_port = listener.getsockname()[1]
    dashboard_env = dict(environment)
    dashboard_env.update({'AGENT_BROWSER_DASHBOARD': '1', 'AGENT_BROWSER_DASHBOARD_BACKEND_ONLY': '1',
                          'AGENT_BROWSER_DASHBOARD_PORT': str(dashboard_port)})
    dashboard_log = (artifacts / 'dashboard-backend.log').open('w')
    dashboard = spawn([source_cli], task_env=dashboard_env, stdout=dashboard_log, stderr=dashboard_log)
    deadline = time.monotonic() + 45
    while True:
        try:
            probe = http.client.HTTPConnection('127.0.0.1', dashboard_port, timeout=1)
            probe.request('GET', '/api/dashboard-auth/status', headers={'Cookie': cookie})
            response = probe.getresponse()
            payload = json.loads(response.read())
            probe.close()
            if response.status == 200 and payload.get('authenticated'):
                break
        except (OSError, ValueError):
            pass
        assert dashboard.poll() is None and time.monotonic() < deadline, 'task dashboard backend did not become authenticated'
        time.sleep(0.05)
    # Exercise the real embedded dashboard after source API and draft checks.
    fresh_status, _, fresh = exchange('/api/service/status', cookie=cookie, method='GET')
    assert fresh_status == 200
    workspace = fresh['data']['browserRuntime']['workspace']
    config = {'root': str(root), 'artifacts': str(artifacts), 'origin': f'http://127.0.0.1:{dashboard_port}',
              'cookie': cookie, 'browserId': browser_id, 'tabs': []}
    # The source host environment is private to its spawn; pass the provider
    # origin through its explicit fixture receipt rather than ambient defaults.
    provider_receipt = json.loads((artifacts / 'managed-provider-origin.json').read_text())
    config['providerOrigin'] = provider_receipt['publicOrigin']
    (artifacts / 'managed-dashboard-address.json').write_text(json.dumps({'port': dashboard_port}))
    config['origin'] = provider_receipt['publicOrigin']
    config['chrome'] = provider_receipt['chrome']
    for lane, page in pages:
        target_id = page['browserSession']['targetId']
        tab = next(tab for tab in workspace['tabs'].values() if tab['targetId'] == target_id)
        label = (tab.get('taskMetadata') or {}).get('taskName') or tab.get('url') or 'Untitled tab'
        config['tabs'].append({'id': tab['id'], 'label': label, 'expectedTitle': expected_targets[target_id]})
    config_path = root / 'compact-ui-config.json'
    config_path.write_text(json.dumps(config))
    config_path.chmod(0o600)
    runner = Path(__file__).resolve().parents[1] / 'test-p237-compact-live.cjs'
    ui = spawn(['node', str(runner), str(config_path)], stdout=-1, stderr=-1)
    stdout, stderr = ui.communicate(timeout=100)
    (artifacts / 'compact-production-ui-run.log').write_bytes(stdout + stderr)
    assert ui.returncode == 0, f'production compact UI failed: {stderr.decode()}'
    assert draft_value('globalThis.__p237Draft') == 'kept across sharing', 'production dashboard reloaded the owned page'
    state_path = root / 'home' / '.agent-browser' / 'service' / 'state.json'
    with (state_path.parent / 'state.json.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        persisted = json.loads(state_path.read_text())
        # Fixture models an existing native route without manufacturing browser,
        # profile, child ownership, or managed generation records.
        for lane, page in pages:
            route_id = f'p237-managed-route:{lane}'
            lease_id = f'p237-managed-controller:{lane}'
            persisted.setdefault('remoteViewRoutes', {})[route_id] = {
                'id': route_id, 'browserId': browser_id, 'sessionId': lane,
                'displayAllocationId': desktop_id, 'state': 'ready',
                'readOnly': False, 'controllerLeaseId': lease_id,
                'controllerEpoch': 1, 'viewerLeaseIds': [lease_id]}
            persisted.setdefault('viewerLeases', {})[lease_id] = {
                'id': lease_id, 'routeId': route_id, 'browserId': browser_id,
                'viewerId': 'dashboard:admin', 'viewerRole': 'controller',
                'state': 'controlling', 'expiresAt': '2099-01-01T00:00:00Z'}
        persisted['stateRevision'] += 1
        temporary = state_path.parent / 'state.managed-operator.tmp'
        temporary.write_text(json.dumps(persisted))
        temporary.chmod(0o600)
        os.replace(temporary, state_path)
        fcntl.flock(lock, fcntl.LOCK_UN)
    preserved = targets()
    # The dashboard's existing auth remains the public ingress boundary.
    before_unauthenticated = foreground()
    denied_status, _, denied = exchange('/api/service/request', {
        'action': 'view_focus', 'browserId': browser_id,
        'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
        'taskName': 'Unauthenticated negative control',
        'params': {'targetId': pages[-1][1]['browserSession']['targetId'], 'operatorFocus': True,
                   'sessionName': pages[-1][0], 'routeId': f'p237-managed-route:{pages[-1][0]}',
                   'controllerLeaseId': f'p237-managed-controller:{pages[-1][0]}'},
    })
    (artifacts / 'managed-public-unauthenticated-denial.json').write_text(json.dumps({'status': denied_status, 'reply': denied}, indent=2) + '\n')
    assert not denied.get('success') and (denied_status in (401, 403) or 'operator_focus' in denied.get('error', '')), 'public focus bypassed dashboard authentication'
    assert foreground() == before_unauthenticated, 'unauthenticated operator focus changed physical selection'
    assert targets() == preserved, 'unauthenticated operator request changed targets'
    observations = []
    for lane, page in reversed(pages):
        target_id = page['browserSession']['targetId']
        status, _, payload = exchange('/api/service/request', {
            'action': 'view_focus', 'browserId': browser_id, 'sessionName': lane,
            'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
            'taskName': 'Select exact managed target', 'jobTimeoutMs': 5000,
            'params': {'targetId': target_id, 'sessionName': lane, 'maximize': True,
                       'operatorFocus': True, 'routeId': f'p237-managed-route:{lane}',
                       'controllerLeaseId': f'p237-managed-controller:{lane}'},
        }, cookie=cookie)
        scene = foreground()
        observations.append({'status': status, 'reply': payload, 'scene': scene, 'targetId': target_id})
        (artifacts / 'managed-public-operator-focus.json').write_text(json.dumps(observations, indent=2) + '\n')
        assert status == 200 and payload.get('success'), f'positive public managed operator focus failed: {payload}'
        assert expected_targets[target_id] in scene['title'], 'managed operator selected the wrong physical target'
        assert targets() == preserved, 'managed operator focus changed peer inventory'
    managed_queued_authority_probe(spawn, browser, pages, exchange, cookie, artifacts, foreground, targets, state_path)


def managed_queued_authority_probe(spawn, browser, pages, exchange, cookie, artifacts, foreground, targets, state_path, native=False):
    """Hold real focus, then invalidate a queued native or legacy proof."""
    import time
    from concurrent.futures import ThreadPoolExecutor
    from urllib.parse import urlsplit

    browser_id = browser['id']
    alice, bob = pages

    def focus(pair):
        lane, page = pair
        started = time.monotonic()
        authority = {} if native else {'routeId': f'p237-managed-route:{lane}',
                                      'controllerLeaseId': f'p237-managed-controller:{lane}'}
        status, _, reply = exchange('/api/service/request', {
            'action': 'view_focus', 'browserId': browser_id, 'sessionName': lane,
            'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
            'taskName': 'Managed queued authority probe', 'jobTimeoutMs': 10000,
            'params': {'targetId': page['browserSession']['targetId'], 'sessionName': lane,
                       'operatorFocus': True, 'maximize': True,
                       **authority},
        }, cookie=cookie)
        return {'status': status, 'reply': reply, 'elapsedSeconds': time.monotonic() - started}

    assert focus(bob)['reply'].get('success')
    preserved = targets()
    endpoint = urlsplit(browser['cdpEndpoint'])
    connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=5)
    try:
        connection.request('GET', '/json/list')
        response = connection.getresponse()
        page = next(page for page in json.loads(response.read()) if page['id'] == alice[1]['browserSession']['targetId'])
    finally:
        connection.close()
    marker = artifacts / ('managed-native-renderer-busy.json' if native else 'managed-renderer-busy.json')
    helper = Path(__file__).resolve().parent / 'p237_cdp_busy.cjs'
    busy = spawn(['node', str(helper), page['webSocketDebuggerUrl'], str(marker)], stdout=-1, stderr=-1)
    deadline = time.monotonic() + 4
    while not marker.exists():
        assert busy.poll() is None and time.monotonic() < deadline, 'managed renderer workload did not start'
        time.sleep(0.01)
    with ThreadPoolExecutor(max_workers=2) as clients:
        older = clients.submit(focus, alice)
        deadline = time.monotonic() + 3
        while 'P237 managed Alice' not in foreground()['title']:
            assert not older.done() and busy.poll() is None and time.monotonic() < deadline, 'older managed focus did not reach its physical effect while busy'
            time.sleep(0.01)
        later = clients.submit(focus, bob)
        # A stale-signature rejection below is the decisive issuance witness;
        # elapsed time is only a scheduling aid, never proof of admission.
        time.sleep(0.3)
        assert not older.done() and not later.done() and busy.poll() is None
        if native:
            generation_override = artifacts / 'managed-provider-viewing-generation.json'
            generation_override.write_text('20\n')
            status, takeover, changed = None, None, {'controllerEpoch': None}
        else:
            status, _, takeover = exchange('/api/service/request', {
                'action': 'service_controller_lease_takeover',
                'serviceName': 'P237ManagedOperatorFixture', 'agentName': 'admin',
                'taskName': 'Replace queued managed controller', 'jobTimeoutMs': 10000,
                'params': {'routeId': f'p237-managed-route:{bob[0]}',
                           'viewerLeaseId': 'p237-managed-controller:replacement'},
            }, cookie=cookie)
            changed = json.loads(state_path.read_text())['remoteViewRoutes'][f'p237-managed-route:{bob[0]}']
        mutation = {'status': status, 'reply': takeover, 'epoch': changed['controllerEpoch'],
                    'nativeViewingGeneration': 20 if native else None,
                    'source': 'synthetic provider generation change' if native else 'public controller takeover',
                    'olderStillRunning': not older.done(), 'laterStillRunning': not later.done()}
        result = later.result(timeout=15)
        first = older.result(timeout=15)
    stdout, stderr = busy.communicate(timeout=5)
    scene = foreground()
    receipt = {'mutation': mutation, 'older': first, 'later': result, 'scene': scene,
               'rendererReturncode': busy.returncode, 'peerInventoryPreserved': targets() == preserved}
    (artifacts / ('managed-native-queued-authority.json' if native else 'managed-queued-authority.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    if native:
        generation_override.unlink()
    assert busy.returncode == 0, f'managed renderer failed: {stderr.decode()}'
    if not native:
        assert status == 200 and takeover.get('success') and changed['controllerEpoch'] == 2, 'public managed controller replacement failed'
    assert first['reply'].get('success'), f'older managed focus failed: {first}'
    assert not result['reply'].get('success') and 'operator_focus_authority_unproven' in result['reply'].get('error', ''), 'later managed focus did not reject its now-stale issued proof'
    assert 'P237 managed Alice' in scene['title'] and targets() == preserved, 'stale managed focus changed the physical target or peers'
