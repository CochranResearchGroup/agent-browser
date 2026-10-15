"""Two actual HTTP clients selecting one desktop while a real renderer is busy."""
import http.client
import fcntl
import hashlib
import json
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.parse import urlsplit


def run_cross_client_focus_fixture(spawn, env, artifacts, state, source_http, cookie, state_path, mode='ordering', native_control=None):
    def scene(session='p237-alpha'):
        scene_env = dict(env)
        if mode == 'independent':
            scene_env['DISPLAY'] = state['browsers'][f'session:{session}']['displayName']
        active = subprocess.check_output(['xdotool','getactivewindow'],env=scene_env,text=True).strip()
        return {'activeWindow':active,
                'inputWindow':subprocess.check_output(['xdotool','getwindowfocus'],env=scene_env,text=True).strip(),
                'title':subprocess.check_output(['xdotool','getwindowname',active],env=scene_env,text=True).strip()}
    def focus(session, require_success=True, timeout_ms=10000):
        tab = next(tab for tab in state['tabs'].values() if tab.get('title') == session)
        started = time.monotonic()
        status, _, reply = source_http('/api/service/request', {
            'action':'view_focus','browserId':f'session:{session}',
            'serviceName':'P237Fixture','agentName':'admin','taskName':f'cross-client-{session}',
            'jobTimeoutMs':timeout_ms,
            'params':{'sessionName':session,'targetId':tab['targetId'],
                      'operatorFocus':True,'maximize':True,
                      'routeId':f'fixture-route:{session}',
                      'controllerLeaseId':f'fixture-controller:{session}'}},cookie,timeout=15)
        result = {'session':session,'startedAt':started,'finishedAt':time.monotonic(),
                  'status':status,'reply':reply}
        (artifacts/f'cross-client-{session}-reply.json').write_text(json.dumps(result,indent=2)+'\n')
        if require_success:
            assert status == 200 and reply['success'], f'cross-client focus failed: {result}'
        return result
    focus('p237-beta')
    before = scene('p237-beta')
    assert 'p237-beta' in before['title']
    browser = state['browsers']['session:p237-alpha']
    endpoint = urlsplit(browser['cdpEndpoint'])
    connection = http.client.HTTPConnection(endpoint.hostname,endpoint.port,timeout=5)
    try:
        connection.request('GET','/json/list')
        response = connection.getresponse()
        assert response.status == 200
        page = next(page for page in json.loads(response.read()) if page.get('title') == 'p237-alpha')
    finally:
        connection.close()
    marker = artifacts/'source-renderer-busy.json'
    helper = Path(__file__).resolve().parent/'p237_cdp_busy.cjs'
    busy = spawn(['node',str(helper),page['webSocketDebuggerUrl'],str(marker)],stdout=-1,stderr=-1)
    deadline = time.monotonic()+5
    while not marker.exists():
        assert busy.poll() is None and time.monotonic() < deadline, 'real renderer workload did not start'
        time.sleep(0.01)
    with ThreadPoolExecutor(max_workers=2) as clients:
        older = clients.submit(focus,'p237-alpha',False)
        deadline = time.monotonic()+4
        alpha_fence_held = None
        if mode == 'independent':
            identity = '\0'.join([env.get('AGENT_BROWSER_RUNTIME_ENVIRONMENT','production'),
                                  'fixture-route:p237-alpha',browser['displayAllocationId']])
            fence_path = state_path.parent/'desktop-input'/'fences'/(hashlib.sha256(identity.encode()).hexdigest()+'.lock')
            while True:
                if fence_path.exists():
                    with fence_path.open('r+') as fence:
                        try:
                            fcntl.flock(fence,fcntl.LOCK_EX|fcntl.LOCK_NB)
                            fcntl.flock(fence,fcntl.LOCK_UN)
                        except BlockingIOError:
                            alpha_fence_held = {'path':str(fence_path),'observedAt':time.monotonic(),
                                                'displayAllocationId':browser['displayAllocationId']}
                            break
                assert not older.done() and busy.poll() is None and time.monotonic() < deadline, 'older native focus did not hold its real route effect fence'
                time.sleep(0.01)
        while True:
            during = scene()
            if 'p237-alpha' in during['title']:
                break
            assert not older.done() and busy.poll() is None and time.monotonic() < deadline, 'native effect did not reach physical Alpha while its renderer was busy'
            time.sleep(0.01)
        assert not older.done() and busy.poll() is None, 'older focus was not still executing at the observed effect'
        newer = clients.submit(focus,'p237-beta',False,1000 if mode == 'timeout' else 10000)
        mutation = None
        if mode in ['authority','effect-authority','cancel']:
            deadline = time.monotonic()+2
            while True:
                current = json.loads(state_path.read_text())
                jobs = [job for job in current['jobs'].values()
                        if job.get('taskName') == 'cross-client-p237-beta' and job.get('state') == 'running']
                if jobs:
                    break
                assert not newer.done() and not older.done() and time.monotonic() < deadline, 'later focus did not enter native execution before authority mutation'
                time.sleep(0.01)
            if mode == 'cancel':
                cancelled = native_control('service_job_cancel','p237-beta',jobId=jobs[-1]['id'])
                mutation = {'transport':'authenticated-native-Unix','reply':cancelled,'jobId':jobs[-1]['id']}
                (artifacts/'source-cross-client-cancel.json').write_text(json.dumps(mutation,indent=2)+'\n')
                assert cancelled.get('success'), 'actual queued native cancellation was not accepted'
            else:
                mutated_session = 'p237-alpha' if mode == 'effect-authority' else 'p237-beta'
                mutated_route = f'fixture-route:{mutated_session}'
                before_route = current['remoteViewRoutes'][mutated_route]
                mutation_started = time.monotonic()
                status, _, takeover = source_http('/api/service/request', {
                    'action':'service_controller_lease_takeover',
                    'serviceName':'P237Fixture','agentName':'admin','taskName':'cross-client-authority-change',
                    'jobTimeoutMs':10000,
                    'params':{'routeId':mutated_route,
                              'viewerLeaseId':'fixture-controller:cross-client-replacement'}},cookie,timeout=15)
                after_route = json.loads(state_path.read_text())['remoteViewRoutes'][mutated_route]
                mutation = {'status':status,'reply':takeover,'before':before_route,'after':after_route,
                            'startedAt':mutation_started,'finishedAt':time.monotonic(),
                            'olderStillExecuting':not older.done(),'newerStillExecuting':not newer.done(),
                            'rendererStillExecuting':busy.poll() is None}
                (artifacts/'source-cross-client-authority.json').write_text(json.dumps(mutation,indent=2)+'\n')
                assert status == 200 and takeover['success'], 'actual public controller mutation failed'
                assert after_route['controllerEpoch'] == before_route['controllerEpoch']+1
                if mode == 'authority':
                    assert not older.done() and not newer.done(), 'controller mutation occurred after native focus completed'
        new_reply = newer.result(timeout=15)
        newer_scene = scene('p237-beta')
        alpha_after_newer = scene() if mode == 'independent' else None
        old_was_running = not older.done()
        old_reply = older.result(timeout=15)
    stdout,stderr = busy.communicate(timeout=5)
    assert busy.returncode == 0, f'real renderer workload failed: {stderr.decode()}'
    final = scene('p237-beta') if mode == 'independent' else scene()
    receipt = {'mode':mode,'before':before,'olderEffectWhileRendererBusy':during,
               'newerReply':new_reply,'sceneAfterNewerReply':newer_scene,
               'olderWasRunningAfterNewerReply':old_was_running,
               'olderReply':old_reply,'rendererReply':json.loads(stdout),
               'final':final,'authorityMutation':mutation,'alphaFenceHeld':alpha_fence_held,
               'alphaSceneAfterIndependentReply':alpha_after_newer,
               'desktopAssignments':{key:{field:browser.get(field) for field in ['pid','displayName','displayAllocationId']}
                                     for key,browser in state['browsers'].items()} if mode == 'independent' else None}
    (artifacts/'source-cross-client-focus.json').write_text(json.dumps(receipt,indent=2)+'\n')
    assert new_reply['startedAt'] > old_reply['startedAt'], 'newer client was not submitted later'
    if mode in ['ordering','effect-authority','independent']:
        assert old_reply['status'] == 200 and old_reply['reply'].get('success') and new_reply['status'] == 200 and new_reply['reply'].get('success'), 'cross-client selection did not complete both requests'
        if mode == 'independent':
            assert old_was_running and new_reply['finishedAt'] < old_reply['finishedAt'], 'independent native desktop waited for the busy peer selection'
            assert alpha_after_newer == during, 'independent Beta focus changed the busy Alpha desktop'
        if mode == 'effect-authority':
            assert not mutation['rendererStillExecuting'], 'controller authority changed during native renderer-bound focus effects'
            assert old_reply['status'] == 200 and old_reply['reply'].get('success'), 'focus lost authority after its physical effect had begun'
        assert 'p237-beta' in final['title'] and final['activeWindow'] == final['inputWindow'], 'older native focus replaced the later client selection'
    else:
        assert old_reply['status'] == 200 and old_reply['reply'].get('success'), 'queued failure disrupted the older native focus'
        assert not new_reply['reply'].get('success'), 'invalid or expired queued focus was accepted'
        if mode == 'authority':
            assert 'operator_focus' in new_reply['reply'].get('error',''), 'queued authority change did not reach operator proof rejection'
            assert new_reply['finishedAt'] > mutation['finishedAt'], 'proof rejected before the real authority change'
        elif mode == 'cancel':
            assert new_reply['reply'].get('data',{}).get('cancelled') is True, 'queued cancellation did not reach its terminal response'
            assert new_reply['finishedAt'] < old_reply['finishedAt'], 'queued cancellation waited for the older focus'
        else:
            assert new_reply['finishedAt'] < old_reply['finishedAt'], 'queued timeout did not expire while older focus was executing'
        assert 'p237-alpha' in newer_scene['title'] and 'p237-alpha' in final['title'], 'rejected queued focus changed the physical scene'
    if mode == 'cancel':
        successor = focus('p237-beta')
        successor_scene = scene()
        assert 'p237-beta' in successor_scene['title'], 'cancelled reservation blocked the next native selection'
        (artifacts/'source-cross-client-cancel-successor.json').write_text(json.dumps({
            'reply':successor,'scene':successor_scene},indent=2)+'\n')
