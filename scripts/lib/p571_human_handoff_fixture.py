"""Real MCP/browser custody with synthetic Native provider transport.

The viewer/gateway input epoch is separately exercised in Remote View. This
fixture proves producer intent, exact retained identity, pause and owner return.
"""
import http.client
import json
import select
import subprocess
import time
from pathlib import Path


def run_human_handoff_fixture(env, artifacts, sockets, spawn, cli, log):
    env['AGENT_BROWSER_PUBLIC_ORIGIN'] = 'https://p571.synthetic.invalid'
    client = spawn([cli, 'mcp', 'serve'], stdin=subprocess.PIPE,
                   stdout=subprocess.PIPE, stderr=log, bufsize=0)
    sequence = 0

    def rpc(method, params):
        nonlocal sequence
        sequence += 1
        client.stdin.write((json.dumps({'jsonrpc':'2.0','id':sequence,'method':method,'params':params})+'\n').encode())
        client.stdin.flush()
        deadline = time.monotonic()+60
        while True:
            assert client.poll() is None, 'MCP source exited before reply'
            assert select.select([client.stdout], [], [], max(0, deadline-time.monotonic()))[0], 'MCP reply deadline'
            line = client.stdout.readline()
            assert line, 'MCP source ended its response stream'
            reply = json.loads(line)
            if reply.get('id') == sequence:
                return reply

    def call(name, args, success=True):
        reply = rpc('tools/call', {'name':name,'arguments':args})
        if 'error' in reply:
            assert not success, reply
            return reply
        result = reply['result']
        payload = json.loads(next(block['text'] for block in result['content'] if block['type']=='text'))
        assert payload['success'] is success, payload
        return payload.get('data', payload)

    def owner(handoff, choice='inspect_control', authenticated=True):
        port = int((sockets/'default.stream').read_text())
        connection = http.client.HTTPConnection('127.0.0.1',port,timeout=20)
        headers = {'Content-Type':'application/json','X-Remote-View-Operator':'synthetic-nonadmin'}
        if authenticated:
            headers['Authorization']='Bearer '+Path(env['AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE']).read_text().strip()
        body = {'slot':1,'generation':19,'choice':choice}
        if choice == 'return_handoff':
            body['operationId']='synthetic-return-'+handoff
        try:
            connection.request('POST',f'/api/native-remote-view/{handoff}/mode',json.dumps(body),headers)
            response = connection.getresponse()
            data = response.read()
            if not authenticated:
                assert response.status == 403
                return
            assert response.status == 200, (response.status, data.decode())
            reply = json.loads(data)
            assert reply['success'], reply
            return reply['data']['nativeControlTarget']
        finally:
            connection.close()

    rpc('initialize', {'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'synthetic-human-handoff','version':'1'}})
    client.stdin.write((json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n').encode())
    client.stdin.flush()
    task = call('browser_open', {'profile':'handoff-fixture','profileMode':'persistent','taskKey':'human-help',
        'serviceName':'HumanHandoffFixture','agentName':'SourceRegression','taskName':'Manual assistance',
        'url':'data:text/html,<title>Human handoff fixture</title><input id="draft" aria-label="Draft">'})
    call('browser_fill', {'taskId':task['taskId'],'selector':'#draft','value':'Original draft'})
    first = call('browser_handoff', {'taskId':task['taskId']})
    assert first['operatorVisible']['state'] == 'ready'
    assert first.get('interaction', {}).get('intent') == 'interactive', 'default handoff reports visibility without requested human interaction'
    assert owner(first['handoffId'])['handoffIntent'] == 'interactive'
    assert owner(first['handoffId'], 'return_handoff')['handoffIntent'] == 'view'
    passive = call('browser_handoff', {'taskId':task['taskId'],'interaction':'view'})
    assert passive['interaction']['automationPaused'] is False
    handoff = call('browser_handoff', {'taskId':task['taskId']})
    assert handoff['operatorVisible']['state'] == 'ready'
    assert handoff['interaction']['intent'] == 'interactive'
    assert handoff['interaction']['automationPaused'] is True
    assert handoff['handoffId'] != passive['handoffId']
    owner(handoff['handoffId'], authenticated=False)
    owner(handoff['handoffId'], 'return_handoff', authenticated=False)
    proof = owner(handoff['handoffId'])
    assert proof['handoffIntent'] == 'interactive'
    assert owner(first['handoffId'], 'return_handoff')['handoffIntent'] == 'view'
    assert owner(handoff['handoffId'])['handoffIntent'] == 'interactive', 'old return cannot resume a later invitation'
    passive_proof = owner(passive['handoffId'])
    assert passive_proof is None or passive_proof['handoffIntent'] == 'view'
    call('browser_fill', {'taskId':task['taskId'],'selector':'#draft','value':'must not arrive'}, success=False)
    repeated = call('browser_handoff', {'taskId':task['taskId']})
    assert repeated['handoffId'] == handoff['handoffId'], 'lost replies must retain the same invitation'
    assert owner(handoff['handoffId'])['targetDigest'] == proof['targetDigest']
    assert owner(handoff['handoffId'], 'return_handoff')['handoffIntent'] == 'view'
    assert owner(handoff['handoffId'], 'return_handoff')['handoffIntent'] == 'view'
    continued = call('browser_continue', {'taskId':task['taskId']})
    for key in ['taskId','browserId','profileId','tabId']:
        assert continued[key] == task[key], key
    assert 'Original draft' in json.dumps(call('browser_snapshot', {'taskId':task['taskId']}))
    call('browser_fill', {'taskId':task['taskId'],'selector':'#draft','value':'Automation resumed'})
    assert 'Automation resumed' in json.dumps(call('browser_snapshot', {'taskId':task['taskId']}))
    call('browser_close', {'taskId':task['taskId']})
    client.stdin.close()
    client.wait(timeout=10)
    (artifacts/'human-handoff-evidence.json').write_text(json.dumps({
        'task':task,'handoff':handoff,'passive':passive,'continued':continued,
        'sameTarget':True,'automationPausedAndResumed':True,'unauthenticatedDenied':True,
        'scope':'Real MCP and owned browser; synthetic Native provider; Native input epoch covered by companion gateway/browser tests'
    }, indent=2)+'\n')
    log.write('runtime-host-workflow: passed human-handoff\n')
    log.flush()
