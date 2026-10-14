"""Real browser expiry/recovery through the source public locator and callback.

The callback's native actor and provider transport are synthetic. This does not
qualify external native authentication or delivered browser pixels.
"""
import http.client
import json
import socket
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from urllib.parse import urlsplit


def run_managed_recovery_fixture(request, root, artifacts, environment, browser,
                                 targets, foreground, spawn, source_cli, restart_host):
    lane = 'p237-managed-recovery'
    original_targets = targets
    preparation = [{'stage': 'before', 'targets': original_targets()}]
    last_browser = environment.get('P237_NATIVE_RECOVERY_LAST_BROWSER') == '1'
    initial_browser = dict(browser)
    latest_browser = dict(browser)
    initial_unmanaged = set()
    startup_urls = set()
    if last_browser:
        state = request(lane, 'browser_session_status')['data']
        managed_ids = {tab['targetId'] for tab in state['tabs'].values()}
        initial_unmanaged = {tab['id'] for tab in preparation[0]['targets']} - managed_ids
        startup_urls = {tab['url'] for tab in preparation[0]['targets'] if tab['id'] in initial_unmanaged}
        def targets():
            nonlocal latest_browser
            state = request(lane, 'browser_session_status')['data']
            owners = [owner for owner in state['browsers'].values() if owner['profileId'] == initial_browser['profileId']]
            assert len(owners) <= 1, 'recovery duplicated a profile owner'
            if not owners:
                return []
            latest_browser = owners[0]
            endpoint = urlsplit(latest_browser['cdpEndpoint'])
            connection = http.client.HTTPConnection(endpoint.hostname, endpoint.port, timeout=5)
            try:
                connection.request('GET', '/json/list')
                reply = connection.getresponse()
                assert reply.status == 200
                return sorted([{'id': tab['id'], 'title': tab['title'], 'url': tab['url']}
                               for tab in json.loads(reply.read()) if tab['type'] == 'page'], key=lambda tab: tab['id'])
            finally:
                connection.close()
    peers = targets()
    preparation.append({'stage': 'after-read', 'targets': original_targets()})
    request(lane, 'browser_session_open', profileId=browser['profileId'])
    preparation.append({'stage': 'after-open', 'targets': original_targets()})
    created = request(lane, 'tab_new', url='data:text/html,<title>P237 retained recovery</title><p>Recovery</p>',
                      taskName='Original recovery task', purpose='Recover the addressed task',
                      originKind='synthetic-fixture')
    original = created['browserSession']
    preparation.append({'stage': 'after-new-tab', 'targets': original_targets()})
    (artifacts / 'recovery-preparation.json').write_text(json.dumps(preparation, indent=2) + '\n')
    if last_browser:
        staging = [{'stage': 'created', 'targets': targets()}]
        request('p237-managed-alice', 'browser_session_close', leaveOpen=False)
        staging.append({'stage': 'alice-closed', 'targets': targets()})
        request('p237-managed-bob', 'browser_session_close', leaveOpen=False)
        staging.append({'stage': 'bob-closed', 'targets': targets()})
        (artifacts / 'last-browser-staging.json').write_text(json.dumps(staging, indent=2) + '\n')
        peers = []
        remaining = request(lane, 'browser_session_status')['data']
        assert set(remaining['tabs']) == {original['tabId']}, 'last-browser staging left another managed task'
        assert {tab['id'] for tab in targets()} == initial_unmanaged | {original['targetId']}, 'last-browser staging changed an untracked native target'
    link_id, tab_id = original['handoffId'], original['tabId']
    now = int(time.time() * 1000)
    configured = request(lane, 'service_remote_view_handoff_resolve', handoffId=link_id,
                         handoffOperation='set_tab_lifetime', tabInactivityMs=60000, activityAtMs=now)
    assert configured['success']
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    backend_env = dict(environment)
    backend_env.update({'AGENT_BROWSER_DASHBOARD': '1', 'AGENT_BROWSER_DASHBOARD_BACKEND_ONLY': '1',
                        'AGENT_BROWSER_DASHBOARD_PORT': str(port)})
    log = (artifacts / 'recovery-dashboard.log').open('w')
    backend = spawn([source_cli], task_env=backend_env, stdout=log, stderr=log)

    def exchange(path, native=False, method='GET'):
        headers = {'Content-Length': '0'}
        if native:
            headers.update({'Authorization': 'Bearer ' + Path(environment['AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE']).read_text().strip(),
                            'X-Remote-View-Operator': 'p237-synthetic-native-operator'})
        connection = http.client.HTTPConnection('127.0.0.1', port, timeout=100)
        try:
            connection.request(method, path, headers=headers)
            reply = connection.getresponse()
            body = reply.read()
            return reply.status, dict(reply.getheaders()), json.loads(body) if body else None
        finally:
            connection.close()

    deadline = time.monotonic() + 30
    while True:
        try:
            if exchange('/api/dashboard-auth/status')[0] == 200:
                break
        except OSError:
            pass
        assert backend.poll() is None and time.monotonic() < deadline, 'recovery dashboard startup failed'
        time.sleep(0.025)

    request(lane, 'browser_session_reap', activityAtMs=now + 60001)
    expired = request(lane, 'browser_session_status')['data']
    assert tab_id not in expired['tabs'], 'configured real task did not expire'
    assert targets() == peers, 'task expiry changed a peer'
    if last_browser:
        deadline = time.monotonic() + 5
        while Path(f"/proc/{initial_browser['pid']}").exists():
            assert time.monotonic() < deadline, 'last target expiry did not end the original browser process'
            time.sleep(0.025)
    restart = restart_host() if restart_host else None
    ready_owner = request(lane, 'browser_session_status')['data']
    assert tab_id not in ready_owner['tabs'] and targets() == peers, 'owner readiness probe changed expired task or peers'
    path = f'/remote-view/{link_id}'
    public_status, headers, _ = exchange(path)
    assert public_status == 303 and urlsplit(headers['Location']).path == f'/handoff/agent-browser/{link_id}', 'same valid link did not reach native recovery'
    assert targets() == peers, 'public read-only locator recreated its target'
    callback = f'/api/native-remote-view/{link_id}/resume'
    denied, _, _ = exchange(callback, method='POST')
    assert denied == 403 and targets() == peers, 'unauthenticated native callback changed the task'
    with ThreadPoolExecutor(max_workers=2) as clients:
        pending = [clients.submit(exchange, callback, True, 'POST') for _ in range(2)]
        replies = [future.result(timeout=100) for future in pending]
    (artifacts / 'managed-recovery-callback-replies.json').write_text(json.dumps(replies, indent=2) + '\n')
    if any(reply[0] != 200 for reply in replies):
        before = targets()
        minimal = request(lane, 'service_remote_view_handoff_resolve', require_success=False,
                          handoffId=link_id, nativeDesktop=True,
                          runtimeProfile=None, profile=None, sessionName=None)
        diagnostic = request(lane, 'service_remote_view_handoff_resolve', require_success=False,
                             handoffId=link_id, nativeDesktop=True)
        (artifacts / 'managed-recovery-owner-diagnostic.json').write_text(json.dumps({
            'minimalWire': minimal, 'reply': diagnostic, 'before': before, 'after': targets(),
            'qualification': 'diagnostic only; does not replace the failed public callback oracle'}, indent=2) + '\n')
    assert all(reply[0] == 200 for reply in replies), f'native recovery callback failed: {[reply[2] for reply in replies]}'
    recovered = request(lane, 'browser_session_status')['data']
    link = recovered['remoteViewTabHandoffs'][link_id]
    new_tab = recovered['tabs'][link['tabId']]
    observed = targets()
    receipt = {'handoffId': link_id, 'oldTabId': tab_id, 'newTabId': new_tab['id'],
               'publicStatus': public_status, 'deniedStatus': denied,
               'callbackStatuses': [reply[0] for reply in replies], 'restart': restart,
               'lastBrowser': last_browser, 'originalBrowser': initial_browser, 'recoveredBrowser': latest_browser,
               'targets': observed, 'scene': foreground(),
               'metadata': recovered['tabMetadata'][new_tab['id']],
               'scope': 'real source locator/callback, SQLite and Chrome; synthetic native actor/provider transport'}
    (artifacts / 'managed-public-recovery.json').write_text(json.dumps(receipt, indent=2) + '\n')
    assert new_tab['id'] != tab_id
    if last_browser:
        assert latest_browser['pid'] != initial_browser['pid'] and latest_browser['profileId'] == initial_browser['profileId'], 'recovery did not wake the exact retained profile'
    else:
        assert new_tab['browserId'] == browser['id'], 'recovery did not reuse its eligible browser'
    if last_browser:
        # The fresh owned Chrome may also publish its normal startup page.
        # Qualify one recreated task, not a promise that Chrome has no chrome UI.
        assert set(recovered['tabs']) == {new_tab['id']}, 'concurrent recovery duplicated an owned task'
        assert sum(tab['id'] == new_tab['targetId'] for tab in observed) == 1
        assert all(tab['id'] == new_tab['targetId'] or tab['url'] in startup_urls for tab in observed), 'recovery created an unrelated native document'
    else:
        assert len(observed) == len(peers) + 1 and all(peer in observed for peer in peers), 'concurrent recovery duplicated a target or changed peers'
    assert 'P237 retained recovery' in foreground()['title'], 'recovered task was not physically selected'
    assert receipt['metadata']['taskName'] == 'Original recovery task' and receipt['metadata']['purpose'] == 'Recover the addressed task'
    assert receipt['metadata']['firstHumanUseAtMs'] is None, 'readiness or callback fabricated native human use'
    assert exchange(path)[0] == 303, 'same opaque link was lost after recovery'
    # Link expiration and revocation remain independent from the live target.
    request(lane, 'service_remote_view_handoff_resolve', handoffId=link_id,
            handoffOperation='set_link_lifetime', ttlMs=1)
    deadline = time.monotonic() + 5
    while exchange(path)[0] != 410:
        assert time.monotonic() < deadline, 'configured link did not expire'
        time.sleep(0.01)
    assert targets() == observed, 'link expiry closed a live task'
    assert exchange(callback, True, 'POST')[0] == 410 and targets() == observed, 'expired link performed recovery'
    request(lane, 'service_remote_view_handoff_resolve', handoffId=link_id, handoffOperation='delete')
    assert exchange(path)[0] == 404 and exchange(callback, True, 'POST')[0] == 404
    assert targets() == observed, 'link deletion closed a live task'
    request(lane, 'browser_session_close', leaveOpen=False)
    assert targets() == peers, 'recovery task cleanup changed peers'
    if last_browser:
        deadline = time.monotonic() + 5
        while Path(f"/proc/{latest_browser['pid']}").exists():
            assert time.monotonic() < deadline, 'final recovery-task close did not end the woken browser'
            time.sleep(0.025)
