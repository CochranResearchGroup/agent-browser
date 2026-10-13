"""Task-owned managed admission through the existing authenticated native protocol."""
import json
import http.client
import socket
import subprocess
import time
from pathlib import Path
from urllib.parse import urlsplit


def run_managed_session_fixture(env, root, artifacts, sockets, manifest, token, workflow_log):
    endpoint = next(path for path in sockets.iterdir() if path.is_socket()
                    and f'unix:{path.stat().st_dev}:{path.stat().st_ino}' == manifest['socketIdentity'])
    profile = root/'managed-profile'
    profile.mkdir(mode=0o700)
    replies = []

    def request(lane, action, **params):
        command = {'id':f'p237-managed-{len(replies)}', 'action':action,
                   '_agentBrowserAuthToken':token, '_agentBrowserRuntimeLane':lane,
                   'sessionName':lane, 'runtimeProfile':'p237-managed-profile',
                   'profile':str(profile), **params}
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as connection:
            connection.settimeout(35)
            connection.connect(str(endpoint))
            connection.sendall((json.dumps(command)+'\n').encode())
            with connection.makefile('rb') as reader:
                reply = json.loads(reader.readline())
        replies.append({'lane':lane,'action':action,'reply':reply})
        (artifacts/'managed-native-replies.json').write_text(json.dumps(replies,indent=2)+'\n')
        assert reply.get('success'), f'managed native {action} failed: {reply}'
        return reply

    alice = request('p237-managed-alice','browser_session_open',profileId='p237-managed-profile')
    alice_page = request('p237-managed-alice','navigate',url='data:text/html,<title>P237 managed Alice</title><p>Alice</p>')
    bob = request('p237-managed-bob','browser_session_open',profileId='p237-managed-profile')
    assert alice['data']['browserId'] == bob['data']['browserId'], 'managed peers created duplicate browsers'
    bob_page = request('p237-managed-bob','navigate',url='data:text/html,<title>P237 managed Bob</title><p>Bob</p>')
    state = request('p237-managed-alice','browser_session_status')['data']
    browser_id = alice['data']['browserId']
    browser = state['browsers'][browser_id]
    pid = browser['pid']
    stat = Path(f'/proc/{pid}/stat').read_text()
    birth = stat[stat.rfind(')')+2:].split()[19]
    (artifacts/'managed-browser-birth.json').write_text(json.dumps({'pid':pid,'start':birth,'browser':browser},indent=2)+'\n')
    def targets():
        endpoint = urlsplit(browser['cdpEndpoint'])
        connection = http.client.HTTPConnection(endpoint.hostname,endpoint.port,timeout=5)
        try:
            connection.request('GET','/json/list')
            response = connection.getresponse()
            assert response.status == 200
            return sorted([{'id':tab['id'],'title':tab['title'],'url':tab['url']}
                           for tab in json.loads(response.read()) if tab['type'] == 'page'],key=lambda tab:tab['id'])
        finally:
            connection.close()
    preserved = targets()
    (artifacts/'managed-target-inventory.json').write_text(json.dumps(preserved,indent=2)+'\n')
    expected_targets = {alice_page['browserSession']['targetId']:'P237 managed Alice',
                        bob_page['browserSession']['targetId']:'P237 managed Bob'}
    assert all(any(tab['id'] == target_id and tab['title'] == title for tab in preserved)
               for target_id,title in expected_targets.items()), 'managed documents missing from Chrome'
    request('p237-managed-alice','browser_session_focus',browserId=browser_id,
            targetId=alice_page['browserSession']['targetId'])
    def foreground():
        active = subprocess.check_output(['xdotool','getactivewindow'],env=env,text=True).strip()
        return {'activeWindow':active,
                'inputWindow':subprocess.check_output(['xdotool','getwindowfocus'],env=env,text=True).strip(),
                'title':subprocess.check_output(['xdotool','getwindowname',active],env=env,text=True).strip()}
    before = foreground()
    assert 'P237 managed Alice' in before['title']
    observations = []
    for lane,title in [('p237-managed-alice','P237 managed Alice'),('p237-managed-bob','P237 managed Bob')]:
        for action in ['title','url']:
            observed = request(lane,action)
            after = foreground()
            observations.append({'lane':lane,'action':action,'before':before,'after':after,'targets':targets()})
            (artifacts/'managed-physical-observations.json').write_text(json.dumps(observations,indent=2)+'\n')
            expected = title if action == 'title' else next(tab['url'] for tab in preserved if tab['title'] == title)
            assert observed['data'][action] == expected, 'managed lane read a peer target'
            assert after == before, 'managed read changed physical foreground'
            assert targets() == preserved, 'managed read changed peer targets'
    selections = []
    for lane,page,title in [('p237-managed-bob',bob_page,'P237 managed Bob'),
                            ('p237-managed-alice',alice_page,'P237 managed Alice')]:
        request(lane,'browser_session_focus',browserId=browser_id,targetId=page['browserSession']['targetId'])
        selected = foreground()
        selections.append({'lane':lane,'foreground':selected})
        (artifacts/'managed-explicit-selection.json').write_text(json.dumps(selections,indent=2)+'\n')
        assert title in selected['title'] and selected['activeWindow'] == selected['inputWindow'], 'explicit managed selection did not activate the addressed task'
    request('p237-managed-alice','browser_session_close',leaveOpen=False)
    assert Path(f'/proc/{pid}').exists() and targets() == [tab for tab in preserved if tab['id'] != alice_page['browserSession']['targetId']], 'managed scoped close failed to preserve Bob and peer targets'
    observed = request('p237-managed-bob','title')
    assert observed['data']['title'] == 'P237 managed Bob', 'closing managed Alice ended Bob'
    request('p237-managed-bob','browser_session_close',leaveOpen=False)
    deadline = time.monotonic()+5
    while Path(f'/proc/{pid}').exists():
        try:
            stat = Path(f'/proc/{pid}/stat').read_text()
        except FileNotFoundError:
            break
        assert stat[stat.rfind(')')+2:].split()[19] != birth or time.monotonic() < deadline, 'last managed close did not exit browser'
        if stat[stat.rfind(')')+2:].split()[19] != birth:
            break
        time.sleep(0.025)
    final = request('p237-managed-alice','browser_session_status')['data']
    assert not final['browsers'] and not final['tabs'] and not final['sessions'], 'managed close left live logical resources'
    workflow_log.write('runtime-host-workflow: passed\n')
    workflow_log.flush()
