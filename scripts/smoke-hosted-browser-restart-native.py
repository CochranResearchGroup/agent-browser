#!/usr/bin/env python3
"""Opt-in public-MCP consumer of the isolated Native624 restart producer.

Requires explicit exact Native source, binaries and fresh private result directory.
Never uses the shared Native installation or launches private account work.
"""
import argparse
import importlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import Request, urlopen
from urllib.error import HTTPError


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native-source', 'native-binary', 'agent-binary', 'chrome', 'vnc-binary', 'vnc-lib', 'results'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--display', type=int, required=True)
    parser.add_argument('--vnc-port', type=int, required=True)
    parser.add_argument('--subnet', required=True)
    args = parser.parse_args()
    args.native_source = args.native_source.resolve()
    assert subprocess.check_output(['git', '-C', str(args.native_source), 'status', '--porcelain'], text=True).strip() == ''
    sys.path.insert(0, str(args.native_source / 'tests'))
    h = importlib.import_module('service_handoff_recovery')
    producer = importlib.import_module('native_restart_acceptance')
    seam = importlib.import_module('native_restart_fixture')
    args.results = args.results.resolve(); args.results.mkdir(mode=0o700, parents=True, exist_ok=False)
    args.binary = args.native_binary
    repo = Path(__file__).resolve().parents[1]

    class Backend(producer.NativeRestartBackend):
        def application_loss(self):
            pid, start = self.old_application
            # The Agent Browser runtime owns and reaps this child; Native's
            # independent observer must see positive kernel absence afterward.
            h.wait(lambda: not Path(f'/proc/{pid}').exists(), 20)
            return {'observed': 'lost', 'processId': pid, 'startToken': start,
                    'reopenRequired': True, 'automaticReplay': False}

    class Acceptance(h.Fixture):
        def configure_fixture(self, config):
            return config.replace('[tools]', '[tools]\nwindow_inventory = { adapter = "native_x11", fallback_to_xdotool = false }\ndesktop_control = { adapter = "native_x11", fallback_to_xdotool = false }')

        def start_owner(self):
            token = self.home / 'secrets/control.token'
            token.write_text(self.token); token.chmod(0o600)
            return super().start_owner()

        def run_consumer_workflow(self):
            self.owner_birth = (self.owner(), h.birth(self.owner()))
            environment = json.loads(h.run(*self.base, 'env', 2, '--json'))
            environment = environment.get('environment', environment)
            peer = subprocess.Popen(['/usr/bin/python3', str(h.REPO / 'tests/service_handoff_native_app.py'), str(self.home), '2'],
                                    env=os.environ | environment, stdout=subprocess.DEVNULL,
                                    stderr=(args.results / 'peer.log').open('w'))
            self.apps.append(peer); self.app_births = {2: (peer.pid, h.birth(peer.pid))}
            h.wait(lambda: (self.home / 'app-2-state.json').exists())
            assignment = self.pre_assignments[0]
            target = self.control({'operation': 'observe_assignment', 'assignment_id': assignment['assignmentId'],
                                   'expected_generation': assignment['generation']})['target']
            outer = self
            exchanges = []
            class Broker(BaseHTTPRequestHandler):
                def log_message(self, *_args):
                    pass
                def do_POST(self):
                    raw = self.rfile.read(int(self.headers['Content-Length']))
                    envelope = json.loads(raw)
                    assert self.path == '/v1/consumer' and envelope['application'] == 'fixture'
                    assert envelope['request']['operation'] not in ('acquire', 'release', 'release_idle')
                    request = Request(f'http://127.0.0.1:{outer.control_port}/v1/consumer', data=raw,
                                      headers={'Authorization': 'Bearer ' + outer.token, 'Content-Type': 'application/json'})
                    try:
                        with urlopen(request, timeout=30) as reply:
                            status, body = reply.status, reply.read()
                    except HTTPError as error:
                        status, body = error.code, error.read()
                    exchanges.append({'request': envelope['request'], 'status': status, 'response': json.loads(body)})
                    h.private_json(args.results / 'native-exchanges.json', exchanges)
                    self.send_response(status); self.send_header('Content-Type', 'application/json')
                    self.end_headers(); self.wfile.write(body)
            broker = ThreadingHTTPServer(('127.0.0.1', 0), Broker)
            self.servers.append(broker)
            threading.Thread(target=broker.serve_forever, daemon=True).start()
            config = {'binary': str(args.agent_binary.resolve()), 'chrome': str(args.chrome.resolve()),
                      'results': str(args.results), 'target': target, 'publicOrigin': self.public,
                      'controlOrigin': f'http://127.0.0.1:{broker.server_port}',
                      'tokenFile': str(self.home / 'secrets/control.token')}
            h.private_json(args.results / 'bridge-config.json', config)
            child = subprocess.Popen(['node', str(repo / 'scripts/smoke-hosted-browser-restart-runner.mjs'),
                                      str(args.results / 'bridge-config.json')], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=(args.results / 'agent.log').open('w'), text=True)
            self.apps.append(child)
            backend = None
            deadline = time.monotonic() + 300
            while time.monotonic() < deadline:
                if not select.select([child.stdout], [], [], 1)[0]:
                    assert child.poll() is None, 'Agent runner exited before completion'
                    continue
                line = child.stdout.readline()
                assert line, 'Agent runner pipe closed before completion'
                request = json.loads(line)
                if request['operation'] == 'peers':
                    value = self.snapshot()['2']
                    assert h.birth(peer.pid) == self.app_births[2][1]
                elif request['operation'] == 'restart':
                    assert backend is None, 'Restart is single-use'
                    proof = request['proof']; pid = proof['pid']
                    assert proof['uid'] == os.getuid() and h.birth(pid) == proof['kernelStart']
                    assert Path(f'/proc/{pid}/exe').resolve() == args.chrome.resolve()
                    assert str(args.results / 'agent') in proof['profilePath']
                    assert ('--user-data-dir=' + proof['profilePath']).encode() in Path(f'/proc/{pid}/cmdline').read_bytes().split(b'\0')
                    self.app_births[1] = (pid, proof['kernelStart'])
                    backend = Backend(self)
                    fixture = seam.RestartFixture(backend, {'task': 'synthetic-restart', 'profile': proof['profilePath']})
                    value = fixture.restart(fixture.binding)
                    h.private_json(args.results / 'native-restart-handoff.json', value)
                elif request['operation'] == 'complete':
                    assert request['passed'], 'Agent consumer checks failed; retained agent receipt'
                    assert backend is not None
                    backend.preserve_peer()
                    child.wait(timeout=15); assert child.returncode == 0
                    self.complete = True
                    return
                else:
                    raise ValueError('Unknown bridge operation')
                child.stdin.write(json.dumps({'ok': True, 'value': value}) + '\n'); child.stdin.flush()
            raise TimeoutError('Combined restart acceptance deadline')

    f = Acceptance(args); failure = cleanup = None
    h.private_json(args.results / 'pair.json', {'agentBinarySha256': h.sha(args.agent_binary),
                                              'nativeBinarySha256': h.sha(args.native_binary),
                                              'fixtureSource': h.run('git', '-C', repo, 'rev-parse', 'HEAD').strip(),
                                              'nativeSource': h.run('git', '-C', args.native_source, 'rev-parse', 'HEAD').strip()})
    try:
        f.setup()
    except Exception as error:
        failure = {'type': type(error).__name__, 'stage': f.stage, 'message': str(error)[:500]}
    finally:
        try: f.cleanup()
        except Exception as error: cleanup = {'type': type(error).__name__, 'message': str(error)[:500]}
    value = {'verdict': 'ACCEPT' if f.complete and not failure and not cleanup else 'INCOMPLETE',
             'failure': failure, 'cleanupFailure': cleanup, 'installationId': f.installation_id}
    h.private_json(args.results / 'combined-receipt.json', value)
    print(json.dumps(value))
    return 0 if value['verdict'] == 'ACCEPT' else 1


if __name__ == '__main__':
    raise SystemExit(main())
