#!/usr/bin/env python3
"""Opt-in P279 same-pair matrix on the existing isolated Native producer fixture.

Creates no shared provider operation. The child owns its Agent Browser runtime;
the Native Fixture owns its two desktops, peer application and resource cleanup.
This adapter never retries a consumer mutation or a restart intent.
"""
import argparse
import base64
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import socketserver
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import traceback
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer



def native_owner_failure_diagnostic(failure):
    """Keep only reviewed infrastructure codes, never the owner's raw suffix."""
    if (not isinstance(failure, dict)
            or type(failure.get('schema_version')) is not int
            or failure['schema_version'] != 1
            or failure.get('code') != 'input_effect_uncertain'
            or failure.get('retryable') is not False):
        return {'reasonCode': 'unclassified', 'parserStage': 'schema_rejected'}
    if not isinstance(failure.get('message'), str) or len(failure['message']) > 4096:
        return {'reasonCode': 'unclassified', 'parserStage': 'message_rejected'}
    token = failure['message'].split(':', 1)[0]
    allowed = {
        'capability_not_configured', 'generation_conflict', 'input_failed',
        'native_x11_display_missing', 'native_x11_auth_missing',
        'native_x11_control_unavailable', 'native_x11_request', 'native_x11_reply',
        'native_x11_grab', 'native_x11_ungrab', 'native_x11_key_state',
        'native_x11_pointer_state', 'native_x11_hold_already_pressed',
        'native_x11_lock_poisoned', 'native_x11_cleanup_uncertain', 'native_x11_release_unconfirmed',
        'input_control_busy', 'input_control_conflict', 'input_control_state_invalid',
        'input_control_lock', 'input_argv_empty', 'input_argv_limit',
        'input_command_incomplete', 'input_argument_invalid',
        'input_command_unsupported', 'input_search_unused',
    }
    return {'reasonCode': token, 'parserStage': 'classified'} if token in allowed else {
        'reasonCode': 'unclassified', 'parserStage': 'unknown_token'}


def load_file(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def assert_viewing_store_clear(value, target):
    """Native routes/sessions use logical IDs; grants also carry viewing IDs."""
    logical_id, viewing_id = target['desktopId'], target['viewingDesktopId']
    target_ids = {logical_id, viewing_id}
    if any(route['desktopId'] in target_ids for route in value['routes'].values()):
        raise ValueError('Owned Native route remains admitted')
    if any(grant['request']['target']['viewingDesktopId'] == viewing_id and not grant['revoked']
           for grant in value.get('consumerGrants', {}).values()):
        raise ValueError('Owned Native grant remains live')
    if any(session['desktopId'] in target_ids and session['state'] == 'admitted'
           for session in value['sessions'].values()):
        raise ValueError('Owned Native session remains admitted')
    return {'routes': 0, 'admittedSessions': 0}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('native-source', 'native-binary', 'agent-binary', 'chrome',
                 'vnc-binary', 'vnc-lib', 'consumer-root', 'node', 'results'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--browser-manifest', type=Path)
    parser.add_argument('--agent-source-commit', required=True)
    parser.add_argument('--native-binary-source-commit')
    parser.add_argument('--challenge-assistance', action='store_true', help='Add owned HTTPS challenge/native assistance to bounded input diagnostic')
    parser.add_argument('--observation-age-probe', action='store_true', help='Characterize current Native motion across the archived observation-age boundary; no checkbox click')
    parser.add_argument('--checkbox-assistance', action='store_true', help='Qualify owned Native checkbox/passive inputs and independent HTTPS admission')
    parser.add_argument('--image-rounds-assistance', action='store_true', help='Qualify two complete owned Native image grids under one original budget')
    parser.add_argument('--image-assistance', action='store_true', help='Qualify one owned static Native image round with agent-inspected pixels')
    parser.add_argument('--checkbox-observe-only', action='store_true', help='Capture the owned synthetic checkbox without challenge input')
    parser.add_argument('--mode-preflight', action='store_true', help='Read public browser mode before first fill; unverified evidence stops the fixture')
    parser.add_argument('--mode-diagnostic-only', action='store_true', help='Stop before fill after the read-only mode preflight regardless of result')
    parser.add_argument('--input-diagnostic', action='store_true', help='Bounded Native input diagnosis only; never full-matrix acceptance')
    parser.add_argument('--display', type=int, required=True)
    parser.add_argument('--vnc-port', type=int, required=True)
    parser.add_argument('--subnet', required=True)
    args = parser.parse_args()
    assert not args.mode_diagnostic_only or args.mode_preflight, 'Mode diagnostic requires read-only preflight'
    assert not args.challenge_assistance or args.input_diagnostic, 'Challenge lane requires bounded input diagnostic'
    assert not args.observation_age_probe or args.challenge_assistance, 'Age probe requires owned challenge fixture'
    assert not args.checkbox_assistance or args.challenge_assistance, 'Checkbox lane requires owned HTTPS challenge fixture'
    assert not args.checkbox_observe_only or args.checkbox_assistance
    assert not (args.checkbox_assistance and args.observation_age_probe), 'Probe and checkbox effects are separate bounded lanes'
    assert not args.image_rounds_assistance or args.image_assistance
    assert not args.image_assistance or args.challenge_assistance
    assert not args.image_assistance or not (args.checkbox_assistance or args.observation_age_probe or args.checkbox_observe_only)
    for commit in (args.agent_source_commit, args.native_binary_source_commit):
        assert commit is None or len(commit) == 40 and all(character in '0123456789abcdef' for character in commit)
    repo = Path(__file__).resolve().parents[1]
    for name in ('native_source', 'native_binary', 'agent_binary', 'chrome',
                 'vnc_binary', 'vnc_lib', 'consumer_root', 'node', 'results'):
        value = getattr(args, name)
        assert value.is_absolute(), name + ' requires an explicit absolute path'
        setattr(args, name, value.resolve())
    if args.browser_manifest:
        assert args.browser_manifest.is_absolute()
        args.browser_manifest = args.browser_manifest.resolve(strict=True)
    for checkout in (repo, args.native_source, args.consumer_root):
        assert subprocess.check_output(['git', '-C', str(checkout), 'status',
                                        '--porcelain', '--untracked-files=no'],
                                       text=True).strip() == '', 'Commit tracked source before qualification'
    assert all(path.is_file() for path in (args.native_binary, args.agent_binary,
                                          args.chrome, args.node, args.vnc_binary))
    args.results.mkdir(mode=0o700, parents=True, exist_ok=False)
    assert args.results.stat().st_uid == os.getuid() and args.results.stat().st_mode & 0o077 == 0
    started = time.monotonic()
    workload_seconds = 300 if args.input_diagnostic else 1200
    workload_deadline = started + workload_seconds
    final_deadline = workload_deadline + 180
    phase_deadline = [workload_deadline]
    sys.path.insert(0, str(args.native_source / 'tests'))
    h = importlib.import_module('service_handoff_recovery')
    original_run, original_wait = h.run, h.wait

    def remaining(limit):
        value = min(limit, phase_deadline[0] - time.monotonic())
        if value <= 0:
            raise TimeoutError('P279 declared total workload/cleanup deadline')
        return value

    def bounded_run(*command, timeout=60, **kwargs):
        return original_run(*command, timeout=remaining(timeout), **kwargs)

    def bounded_wait(predicate, seconds=15):
        return original_wait(predicate, remaining(seconds))

    h.run, h.wait = bounded_run, bounded_wait
    producer = importlib.import_module('native_restart_acceptance')
    seam = importlib.import_module('native_restart_fixture')
    prior = load_file('p279_existing_restart_adapter', repo / 'scripts/smoke-hosted-browser-restart-native.py')
    transport = load_file('p279_native_rpc', repo / 'scripts/hosted-native-rpc.py')
    args.binary = args.native_binary
    fixture_files = ('scripts/smoke-hosted-browser-matrix-native.py', 'scripts/hosted-native-rpc.py',
                     'scripts/smoke-hosted-browser-attachment.mjs', 'scripts/hosted-performance.mjs')
    if args.challenge_assistance:
        fixture_files += ('scripts/qualify-native-challenge-assistance.mjs', 'scripts/qualify-native-no-cdp-challenge.mjs', 'packages/client/src/challenge-assistance.js')
    if args.checkbox_assistance:
        fixture_files += ('scripts/qualify-native-checkbox.mjs',)
    if args.image_assistance:
        fixture_files += ('scripts/qualify-native-image.mjs', 'scripts/qualify-native-checkbox.mjs',
                          'scripts/probe-native-pointer.py', 'scripts/test-native-owner-failure-diagnostics.py',
                          'packages/client/src/native-image-challenge.js', 'packages/client/src/native-image-challenge.d.ts')
    if args.image_rounds_assistance:
        fixture_files += ('scripts/qualify-native-image-rounds.mjs',)
    for name in fixture_files:
        h.run('git', '-C', repo, 'ls-files', '--error-unmatch', name)
    source_pair = {'agentBinarySha256': h.sha(args.agent_binary), 'agentBinarySourceCommit': args.agent_source_commit,
                   'nativeBinarySha256': h.sha(args.native_binary), 'nativeBinarySourceCommit': args.native_binary_source_commit,
                   'chromeSha256': h.sha(args.chrome), 'consumerSourceCommit': h.run('git', '-C', args.consumer_root, 'rev-parse', 'HEAD').strip(),
                   'nativeFixtureSourceCommit': h.run('git', '-C', args.native_source, 'rev-parse', 'HEAD').strip(),
                   'harnessSourceCommit': h.run('git', '-C', repo, 'rev-parse', 'HEAD').strip(),
                   'fixtureFiles': {name: h.sha(repo / name) for name in fixture_files},
                   'fixtureTransport': {'nativePublicHost': 'native.p279.example.invalid',
                                        'appPublicHost': 'app.p279.example.invalid',
                                        'nativeConnectHost': '127.0.0.1', 'scheme': 'https',
                                        'scope': 'Private fixture mapping only; no shared DNS or hosts edits'}}
    pair_digest = hashlib.sha256(json.dumps(source_pair, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
    plan = {'version': 1, 'runId': secrets.token_hex(16), 'pairDigest': pair_digest, 'sourcePair': source_pair,
            'coldContexts': 5, 'publicControlPairs': 10, 'nativeControlPairs': 3, 'warmTargetMs': 250,
            'functionalUiPairs': 1, 'nativeTextEdits': 1, 'deadlineSeconds': 1200, 'cleanupDeadlineSeconds': 180, 'challengeAssistance': args.challenge_assistance, 'modePreflight':args.mode_preflight, 'modeDiagnosticOnly':args.mode_diagnostic_only}
    plan.update(checkboxAssistance=args.checkbox_assistance, imageAssistance=args.image_assistance, imageRoundsAssistance=args.image_rounds_assistance, checkboxObserveOnly=args.checkbox_observe_only, observationAgeProbe=args.observation_age_probe)
    if args.input_diagnostic:
        plan.update(mode='native_input_diagnostic', coldContexts=1, publicControlPairs=0,
                    nativeControlPairs=0, deadlineSeconds=300)
    # Frozen before the creating Native Fixture can install or start anything.
    h.private_json(args.results / 'workload-plan.json', plan)
    lock = threading.RLock()
    websocket_lock = threading.Lock()
    websocket_connections = set()
    exchange_events, bridge_events, adapter_errors = [], [], []
    native_input_events, native_refusals = [], []
    bridge_token = secrets.token_hex(32)
    socket_root = Path(tempfile.mkdtemp(prefix='p279-native-rpc-'))
    socket_root.chmod(0o700)
    socket_path = socket_root / 'native.sock'
    bridge_token_path = args.results / 'restart-bridge.token'
    bridge_token_path.write_text(bridge_token)
    bridge_token_path.chmod(0o600)

    class Backend(producer.NativeRestartBackend):
        def application_loss(self):
            pid, birth = self.old_application
            h.wait(lambda: not Path(f'/proc/{pid}').exists(), 20)
            return {'observed': 'lost', 'processId': pid, 'startToken': birth,
                    'reopenRequired': True, 'automaticReplay': False}

    class Acceptance(h.Fixture):
        consumer_capabilities = (*h.Fixture.consumer_capabilities, 'windows', 'activate', 'events', 'release')

        def __init__(self, parsed):
            super().__init__(parsed)
            self.public = f'https://native.p279.example.invalid:{self.listener.server_port}'
            self.app_origin = f'https://app.p279.example.invalid:{h.free_port()}'
            self.native_connect_origin = f'https://127.0.0.1:{self.listener.server_port}'
            h.private_json(parsed.results / 'fixture-origin-transport.json',
                           {'nativePublicOrigin': self.public, 'appPublicOrigin': self.app_origin,
                            'nativeConnectOrigin': self.native_connect_origin,
                            'scope': 'Private fixture transport; no shared DNS, hosts, authentication or ingress changes'})
            self.rpc = None
            self.extra_servers = []
            self.agent_child = None
            self.restart_admitted = False
            self.restart_backend = None
            self.bridge_ids = set()
            certificate, key = parsed.results / 'fixture.crt', parsed.results / 'fixture.key'
            try:
                h.run('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1',
                      '-keyout', key, '-out', certificate, '-subj', '/CN=127.0.0.1',
                      '-addext', 'subjectAltName=IP:127.0.0.1,DNS:native.p279.example.invalid', timeout=20)
                key.chmod(0o600)
                context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
                context.load_cert_chain(certificate, key)
                self.listener.socket = context.wrap_socket(self.listener.socket, server_side=True)
                self.certificate = certificate
            except BaseException:
                # Neither server has started; close both exact constructor-owned
                # listeners even though no complete Fixture can be returned.
                self.listener.server_close()
                self.auth.server_close()
                raise

        def configure_fixture(self, config):
            config = config.replace('[consumers.fixture]', '[consumers.fixture]\nselective_input = true\nembed_origins = ' + json.dumps([self.app_origin]))
            return config.replace('[tools]', '[tools]\nwindow_inventory = { adapter = "native_x11", fallback_to_xdotool = false }\ndesktop_control = { adapter = "native_x11", fallback_to_xdotool = false }')

        def start_owner(self):
            token = self.home / 'secrets/control.token'
            token.write_text(self.token)
            token.chmod(0o600)
            return super().start_owner()

        def handler(self):
            parent = super().handler()
            class ViewingProxy(parent):
                def handle_request(request):
                    if request.path.startswith('/fixture/') and request.path != '/fixture/login':
                        return request.local(403, b'Only the synthetic login path is exposed')
                    websocket = request.headers.get('Upgrade', '').lower() == 'websocket'
                    connection = id(request.connection)
                    if websocket:
                        with websocket_lock:
                            websocket_connections.add(connection)
                    try:
                        return super().handle_request()
                    finally:
                        if websocket:
                            with websocket_lock:
                                websocket_connections.discard(connection)
            return ViewingProxy

        def assert_viewing_clear(self):
            with websocket_lock:
                assert not websocket_connections, 'Native WebSocket transport remains open'
            value = json.loads((self.home / 'control/viewing.json').read_text())
            return {'webSockets': 0, **assert_viewing_store_clear(value, self.target)}

        def peer(self):
            assert h.birth(self.peer_process.pid) == self.app_births[2][1]
            value = self.snapshot()['2']
            assert value == self.peer_before, 'Native peer desktop changed'
            assert self.protected() == self.protected_before, 'Native protected configuration changed'
            return {'native': value, 'application': {'pid': self.peer_process.pid,
                    'start': self.app_births[2][1]}}

        def bridge(self, operation, body):
            request_id = body['requestId']
            assert isinstance(request_id, str) and request_id and request_id not in self.bridge_ids
            self.bridge_ids.add(request_id)
            if operation == 'peers':
                return self.peer()
            if operation == 'restart':
                assert not self.restart_admitted, 'One admitted restart; no effect replay'
                self.restart_admitted = True
                h.private_json(args.results / 'matrix-restart-intent.json', body)
                proof = body['proof']
                profile = Path(proof['profilePath']).resolve(strict=True)
                assert profile.is_relative_to(args.results / 'agent')
                self.restart_profile = profile
                assert proof['uid'] == os.getuid() and h.birth(proof['pid']) == proof['kernelStart']
                assert Path(f"/proc/{proof['pid']}/exe").resolve() == args.chrome
                assert prior.exact_profile_argument(Path(f"/proc/{proof['pid']}/cmdline").read_bytes(), profile)
                h.wait(lambda: not websocket_connections, 20)
                # Retirement is exact, one-shot and uses only grants tracked by
                # the existing scoped Native adapter; no arbitrary route IDs.
                self.rpc.reconcile_retired_grants(json.loads((self.home / 'control/viewing.json').read_text()))
                self.rpc.retire_grants()
                scope = self.rpc.assert_restart_clear()
                independent = self.assert_viewing_clear()
                h.private_json(args.results / 'pre-restart-custody.json', {'scope': scope, 'independent': independent})
                self.app_births[1] = (proof['pid'], proof['kernelStart'])
                self.restart_backend = Backend(self)
                fixture = seam.RestartFixture(self.restart_backend, {'task': 'P279 managed restart', 'profile': str(profile)})
                result = fixture.restart(fixture.binding)
                self.rpc.advance_viewing_fence(result['before'], result['after'])
                h.private_json(args.results / 'matrix-native-restart.json', result)
                return result
            if operation == 'reopened':
                assert self.restart_backend is not None
                current, proof = self.restart_backend.observe(), body['proof']
                assert proof['uid'] == os.getuid() and h.birth(proof['pid']) == proof['kernelStart']
                assert Path(f"/proc/{proof['pid']}/exe").resolve() == args.chrome
                assert prior.exact_profile_argument(Path(f"/proc/{proof['pid']}/cmdline").read_bytes(), proof['profilePath'])
                assert Path(proof['profilePath']).resolve().is_relative_to(args.results / 'agent')
                assert Path(proof['profilePath']).resolve() == self.restart_profile
                windows = self.control({'operation': 'windows', 'assignment_id': current['assignmentId'],
                                        'expected_generation': current['lifecycleGeneration'],
                                        'expected_viewing_generation': current['viewingGeneration']})
                assert any(window.get('pid') == proof['pid'] for window in windows['windows'])
                self.peer()
                return {'desktopId': current['desktopId'], 'viewingGeneration': current['viewingGeneration'],
                        'processId': proof['pid'], 'windowObserved': True}
            raise ValueError('Unknown restart bridge operation')

        def run_consumer_workflow(self):
            self.owner_birth = (self.owner(), h.birth(self.owner()))
            environment = json.loads(h.run(*self.base, 'env', 2, '--json'))
            environment = environment.get('environment', environment)
            self.peer_process = subprocess.Popen(['/usr/bin/python3', str(h.REPO / 'tests/service_handoff_native_app.py'), str(self.home), '2'],
                                                 env=os.environ | environment, stdout=subprocess.DEVNULL,
                                                 stderr=(args.results / 'peer.log').open('w'))
            self.apps.append(self.peer_process)
            self.app_births = {2: (self.peer_process.pid, h.birth(self.peer_process.pid))}
            h.wait(lambda: (self.home / 'app-2-state.json').exists())
            self.peer_before, self.protected_before = self.snapshot()['2'], self.protected()
            assignment = self.pre_assignments[0]
            self.target = self.control({'operation': 'observe_assignment', 'assignment_id': assignment['assignmentId'],
                                        'expected_generation': assignment['generation']})['target']
            def record(event):
                exchange_events.append(event)
                h.private_json(args.results / 'scoped-native-exchanges.json', exchange_events)
            self.rpc = transport.ScopedNativeRpc(f'http://127.0.0.1:{self.control_port}', self.token,
                                                 assignment, self.target['viewingDesktopId'], record=record,
                                                 embed_origin=self.app_origin,
                                                 viewing_generation=self.target['viewingGeneration'])
            for name in ('handle', 'handle_consumer', 'retire_grants', 'assert_restart_clear', 'advance_viewing_fence'):
                assert callable(getattr(self.rpc, name, None)), 'Scoped Native adapter API missing: ' + name
            outer = self
            class UnixRequest(socketserver.StreamRequestHandler):
                def handle(request):
                    request.connection.settimeout(remaining(120))
                    while True:
                        raw = request.rfile.readline(65537)
                        if not raw:
                            return
                        assert len(raw) <= 65536 and raw.endswith(b'\n'), 'Bounded Native RPC frame required'
                        message = json.loads(raw)
                        try:
                            with lock:
                                response = outer.rpc.handle(message)
                                call = message.get('params', {})
                                consumer = call.get('arguments', {}).get('request', {})
                                if (args.image_assistance and call.get('name') == 'remote_view_consumer'
                                        and consumer.get('operation') == 'launch_environment'):
                                    launch = outer.rpc._value(response)
                                    assert launch['schemaVersion'] == 1 and launch['readinessScope'] == 'live_resource'
                                    for field in ('assignmentId','desktopId','lifecycleGeneration','viewingDesktopId','viewingGeneration'):
                                        assert launch['target'][field] == outer.target[field]
                                    # Reuse the same scoped public Native response as
                                    # the child; cookie-file locators stay memory-only.
                                    outer.image_pointer_environment = {key:launch['environment'][key]
                                                                       for key in ('DISPLAY','XAUTHORITY')}

                        except Exception as error:
                            adapter_errors.append({'transport': 'unix', 'type': type(error).__name__})
                            response = {'jsonrpc': '2.0', 'id': message.get('id'),
                                        'error': {'code': -32000, 'message': 'Scoped Native fixture refused; inspect evidence'}}
                        if 'id' in message:
                            request.wfile.write(json.dumps(response).encode() + b'\n')
                            request.wfile.flush()
            class UnixServer(socketserver.ThreadingUnixStreamServer):
                daemon_threads = True
            unix = UnixServer(str(socket_path), UnixRequest)
            socket_path.chmod(0o600)
            self.extra_servers.append(unix)
            threading.Thread(target=unix.serve_forever, daemon=True).start()
            class BridgeRequest(BaseHTTPRequestHandler):
                def log_message(self, *_args):
                    pass
                def do_POST(request):
                    try:
                        forwarded_status = 200
                        assert request.headers.get('Authorization') == 'Bearer ' + bridge_token
                        length = int(request.headers.get('Content-Length', '0'))
                        assert 0 < length <= 65536
                        body = json.loads(request.rfile.read(length))
                        with lock:
                            if request.path in ('/v1/agent-desktop/observe', '/v1/agent-desktop/ocr', '/v1/agent-desktop/capture_region', '/v1/agent-desktop/control', '/v1/agent-desktop/input'):
                                assert args.challenge_assistance
                                assert not request.path.endswith('/control') or body.get('operation') == 'status'
                                assert body['desktopId'] == outer.target['viewingDesktopId'] and body['expectedGeneration'] == outer.target['viewingGeneration']
                                if request.path.endswith('/input'):
                                    assert not args.checkbox_observe_only
                                    actions = body['actions']
                                    motion = args.observation_age_probe and 0 < len(actions) <= 16 and all(set(action) == {'kind', 'x', 'y'} and action['kind'] == 'move' for action in actions)
                                    action = actions[0] if len(actions) == 1 else {}
                                    visual = body.get('expectedVisual', {})
                                    checkbox = args.checkbox_assistance and len(actions) == 1 and set(action) == {'kind', 'x', 'y', 'button', 'count'} and action['kind'] == 'click' and action['button'] == 1 and action['count'] == 1 and type(body.get('controlEpoch')) is int and body['controlEpoch'] >= 0 and isinstance(visual.get('sha256'), str) and len(visual['sha256']) == 64 and all(c in '0123456789abcdef' for c in visual['sha256']) and isinstance(visual.get('region'), dict)
                                    image = args.image_assistance and 2 <= len(actions) <= 5 and all(set(item) == {'kind', 'x', 'y', 'button', 'count'} and item['kind'] == 'click' and item['button'] == 1 and item['count'] == 1 for item in actions) and type(body.get('controlEpoch')) is int and body['controlEpoch'] >= 0 and isinstance(visual.get('sha256'), str) and len(visual['sha256']) == 64 and all(c in '0123456789abcdef' for c in visual['sha256']) and isinstance(visual.get('region'), dict)
                                    assert motion or checkbox or image
                                    native_input_events.append({'state':'forwarding_once','windowId':body['windowId'],'nativeControlEpoch':body.get('controlEpoch'),
                                        'generation':body['expectedGeneration'],'actionKinds':[action['kind'] for action in actions],'expectedVisualSha256':visual.get('sha256')})
                                    h.private_json(args.results / 'native-input-forwarding.json', native_input_events)
                                from urllib.request import Request, build_opener, HTTPRedirectHandler
                                class NoRedirect(HTTPRedirectHandler):
                                    def redirect_request(self, *_args, **_kwargs):
                                        return None
                                forwarded = Request(f'http://127.0.0.1:{outer.control_port}'+request.path, data=json.dumps(body).encode(), method='POST', headers={'Authorization':'Bearer '+outer.token,'Content-Type':'application/json'})
                                from urllib.error import HTTPError
                                def retain_visual(label, value):
                                    png = base64.b64decode(value['image']['data'], validate=True)
                                    assert 0 < len(png) <= 4_194_304
                                    assert hashlib.sha256(png).hexdigest() == value['capture']['sha256']
                                    # Each scene/observation remains recoverable when
                                    # later scenarios use the same diagnostic label.
                                    label += '-' + value['capture']['sha256']
                                    h.private_json(args.results / (label + '.json'), value)
                                    path = args.results / (label + '.png')
                                    path.write_bytes(png)
                                    path.chmod(0o600)
                                def guard_observation(label):
                                    # Observe the original guard region without changing,
                                    # refreshing or replaying the input intent.
                                    observation = {key:value for key,value in body.items()
                                                   if key not in ('actions', 'leaseId', 'expectedVisual')}
                                    observation['region'] = body['expectedVisual']['region']
                                    query = Request(f'http://127.0.0.1:{outer.control_port}/v1/agent-desktop/capture_region',
                                                    data=json.dumps(observation).encode(), method='POST',
                                                    headers={'Authorization':'Bearer '+outer.token,'Content-Type':'application/json'})
                                    with build_opener(NoRedirect).open(query, timeout=15) as response:
                                        retain_visual(label, json.load(response))
                                if (args.checkbox_assistance or args.image_assistance) and request.path.endswith('/input'):
                                    h.private_json(args.results / 'native-original-input-intent.json', body)
                                    guard_observation('native-guard-before-input')
                                def pointer_observation(label, observation=None):
                                    if not args.image_assistance or (observation is None and not request.path.endswith('/input')):
                                        return
                                    frame = observation if observation is not None else body
                                    probe = {'processIdentity':frame['processIdentity'] if observation is not None else body['expectedProcessIdentity'],
                                             'windowId':frame['windowId'], 'ownedRoot':str(args.results),
                                             'environment':outer.image_pointer_environment,
                                             'displays':[args.display,args.display+1]}
                                    observed = subprocess.run([sys.executable, str(Path(__file__).with_name('probe-native-pointer.py'))],
                                        input=json.dumps(probe), text=True, capture_output=True, timeout=8)
                                    assert observed.returncode == 0, 'owned_pointer_observation_failed'
                                    value = json.loads(observed.stdout)
                                    value.update(desktopId=frame['desktopId'],
                                                 generation=frame['generation'] if observation is not None else body['expectedGeneration'],
                                                 nativeControlEpoch=body.get('controlEpoch'),phase=label)
                                    h.private_json(args.results / (label+'.json'), value)
                                    assert value.get('status') == 'observed', 'owned_pointer_observation_failed'
                                    if observation is not None:
                                        assert value['button1Down'] is False, 'owned_pointer_already_held'
                                pointer_observation('native-pointer-before-input')
                                try:
                                    with build_opener(NoRedirect).open(forwarded, timeout=15) as result: result=json.load(result)
                                    if (args.checkbox_assistance or args.image_assistance) and request.path.endswith('/ocr'):
                                        retain_visual('native-original-ocr', result)
                                        pointer_observation('native-pointer-original-observation', result)
                                    if args.checkbox_observe_only and request.path.endswith('/ocr'):
                                        h.private_json(args.results / 'synthetic-checkbox-ocr.json', result)
                                        png = base64.b64decode(result['image']['data'], validate=True)
                                        assert 0 < len(png) <= 4_194_304
                                        png_path = args.results / 'synthetic-checkbox-observed.png'
                                        png_path.write_bytes(png)
                                        png_path.chmod(0o600)
                                except HTTPError as error:
                                    # Retain only the owner's bounded diagnostic code,
                                    # never recognized application text or credentials.
                                    failure = json.load(error)
                                    pointer_observation('native-pointer-after-refusal')
                                    code = failure.get('code', failure.get('error', {}).get('code'))
                                    if (args.checkbox_assistance or args.image_assistance) and request.path.endswith('/input') and code == 'visual_observation_conflict':
                                        native_refusals.append({'path':request.path,'status':error.code,'code':code,'effectsAcknowledged':False})
                                        h.private_json(args.results / 'native-owner-refusals.json', native_refusals)
                                        guard_observation('native-guard-after-refusal')
                                        result, forwarded_status = failure, error.code
                                    else:
                                        adapter_errors.append({'transport':'native-owner','path':request.path,'status':error.code,'code':code,
                                            **native_owner_failure_diagnostic(failure)})
                                        h.private_json(args.results / 'matrix-adapter-errors.json', adapter_errors)
                                        raise
                                if (args.checkbox_assistance or args.image_assistance) and request.path.endswith('/input') and forwarded_status == 200:
                                    native_input_events[-1]['ownerDelivery'] = result.get('delivery')
                                    h.private_json(args.results / 'native-input-forwarding.json', native_input_events)
                                    fault_path = args.results / ('agent/native-image-fault.json' if args.image_assistance else 'agent/native-checkbox-fault.json')
                                    if fault_path.exists():
                                        assert not fault_path.is_symlink() and fault_path.resolve().is_relative_to(args.results.resolve())
                                        fault = json.loads(fault_path.read_text())
                                        if fault == {'state':'armed','kind':'drop_input_reply_once'}:
                                            assert result.get('delivery') == 'dispatched'
                                            h.private_json(fault_path, {'state':'consumed','kind':'drop_input_reply_once','ownerDelivery':'dispatched'})
                                            request.close_connection = True
                                            return
                            elif request.path == '/v1/consumer':
                                assert set(body) == {'application', 'request'} and body['application'] == 'fixture'
                                result = outer.rpc.handle_consumer(body['request'])
                            else:
                                operation = request.path.removeprefix('/')
                                bridge_events.append({'operation': operation, 'requestId': body.get('requestId'), 'state': 'received_once'})
                                h.private_json(args.results / 'restart-bridge-events.json', bridge_events)
                                result = outer.bridge(operation, body)
                                result = {'ok': True, 'value': result}
                        encoded = json.dumps(result).encode()
                        request.send_response(forwarded_status)
                    except transport.ConsumerRefused as error:
                        encoded = json.dumps(error.value).encode()
                        request.send_response(400)
                    except Exception as error:
                        adapter_errors.append({'transport': 'http', 'path': request.path, 'type': type(error).__name__})
                        h.private_json(args.results / 'matrix-adapter-errors.json', adapter_errors)
                        encoded = json.dumps({'ok': False, 'retryPermitted': False, 'code': 'fixture_scope_or_effect_unknown'}).encode()
                        request.send_response(502)
                    request.send_header('Content-Type', 'application/json')
                    request.send_header('Content-Length', str(len(encoded)))
                    request.end_headers()
                    request.wfile.write(encoded)
            bridge = ThreadingHTTPServer(('127.0.0.1', 0), BridgeRequest)
            self.extra_servers.append(bridge)
            threading.Thread(target=bridge.serve_forever, daemon=True).start()
            auth = {'cookies': [{'name': 'rv283', 'value': self.cookie, 'domain': 'native.p279.example.invalid',
                                'path': '/', 'expires': -1, 'httpOnly': True, 'secure': True, 'sameSite': 'Strict'}], 'origins': []}
            h.private_json(args.results / 'auth-state.json', auth)
            environment = {key: value for key, value in os.environ.items()
                           if not key.startswith(('AGENT_BROWSER_', 'XDG_', 'HOSTED_SMOKE_'))}
            environment.update({'HOSTED_SMOKE_AB_BINARY': str(args.agent_binary), 'HOSTED_SMOKE_CHROME': str(args.chrome),
                 'HOSTED_SMOKE_BOTS_ROOT': str(args.consumer_root), 'HOSTED_SMOKE_INSTALLATION_ID': self.installation_id,
                 'HOSTED_SMOKE_APP_ORIGIN': self.app_origin, 'HOSTED_SMOKE_NATIVE_ORIGIN': self.public,
                 'HOSTED_SMOKE_NATIVE_APPLICATION': 'fixture', 'HOSTED_SMOKE_NATIVE_SOCKET': str(socket_path),
                 'HOSTED_SMOKE_NATIVE_CONTROL_ORIGIN': f'http://127.0.0.1:{bridge.server_port}',
                 'HOSTED_SMOKE_NATIVE_CONTROL_TOKEN_FILE': str(bridge_token_path),
                 'HOSTED_SMOKE_RESTART_BRIDGE': f'http://127.0.0.1:{bridge.server_port}',
                 'HOSTED_SMOKE_RESTART_TOKEN_FILE': str(bridge_token_path), 'HOSTED_SMOKE_AUTH_STATE': str(args.results / 'auth-state.json'),
                 'HOSTED_SMOKE_WORKLOAD_PLAN': str(args.results / 'workload-plan.json'), 'HOSTED_SMOKE_FIXTURE_TLS': '1',
                 'HOSTED_SMOKE_FIXTURE_NATIVE_CONNECT_ORIGIN': self.native_connect_origin,
                 'HOSTED_SMOKE_CONTINUITY_REQUIRED': '1', 'HOSTED_SMOKE_LOSS_REQUIRED': '1',
                 'HOSTED_SMOKE_EVIDENCE_DIR': str(args.results / 'agent'), 'NODE_EXTRA_CA_CERTS': str(self.certificate)})
            if args.challenge_assistance:
                environment.update({'HOSTED_SMOKE_CHALLENGE_469':'1','HOSTED_SMOKE_CHALLENGE_SITE_CERT':str(self.certificate),'HOSTED_SMOKE_CHALLENGE_SITE_KEY':str(args.results/'fixture.key')})
            if args.observation_age_probe:
                environment['HOSTED_SMOKE_OBSERVATION_AGE_PROBE']='1'
            if args.checkbox_assistance:
                environment['HOSTED_SMOKE_NATIVE_CHECKBOX']='1'
            if args.image_assistance:
                environment['HOSTED_SMOKE_NATIVE_IMAGE']='1'
            if args.image_rounds_assistance:
                environment['HOSTED_SMOKE_NATIVE_IMAGE_ROUNDS']='1'
            if args.checkbox_observe_only:
                environment['HOSTED_SMOKE_NATIVE_CHECKBOX_OBSERVE_ONLY']='1'
            if args.mode_preflight:
                environment['HOSTED_SMOKE_MODE_PREFLIGHT']='1'
            if args.mode_diagnostic_only:
                environment['HOSTED_SMOKE_MODE_DIAGNOSTIC_ONLY']='1'
            if args.browser_manifest:
                environment['HOSTED_SMOKE_BROWSER_MANIFEST'] = str(args.browser_manifest)
            self.agent_child = subprocess.Popen([str(args.node), str(repo / 'scripts/smoke-hosted-browser-attachment.mjs')],
                                                cwd=repo, env=environment, stdin=subprocess.DEVNULL,
                                                stdout=(args.results / 'agent.stdout').open('w'), stderr=(args.results / 'agent.stderr').open('w'))
            self.apps.append(self.agent_child)
            self.agent_child.wait(timeout=remaining(workload_seconds))
            assert self.agent_child.returncode == (1 if args.input_diagnostic else 0), 'Unexpected child outcome; retain every receipt'
            if args.input_diagnostic:
                diagnostic = json.loads((args.results / 'agent/native-input-diagnostic.json').read_text())
                assert diagnostic['passed'] is True and diagnostic['fullMatrix'] is False
                if args.checkbox_assistance and not args.checkbox_observe_only:
                    assert json.loads((args.results / 'agent/checkbox470-acceptance.json').read_text())['passed'] is True
                    assert any(item['code'] == 'visual_observation_conflict' for item in native_refusals), 'Actual pinned Native must refuse a changed frame'
                    assert json.loads((args.results / 'agent/native-checkbox-fault.json').read_text())['state'] == 'consumed'
                if args.image_assistance:
                    image = json.loads((args.results / ('agent/image472-acceptance.json' if args.image_rounds_assistance else 'agent/image471-acceptance.json')).read_text())
                    assert image['passed'] is True and image['actualNativeInput'] is True
                    assert any(event.get('ownerDelivery') == 'dispatched' for event in native_input_events), 'Image acceptance requires an actual Native owner acknowledgement'
            else:
                assert self.restart_backend is not None and self.restart_admitted
                summary = json.loads((args.results / 'agent/qualification-summary.json').read_text())
                assert summary['passed'] and summary['functionalPassed'] and summary['performancePassed']
            assert not adapter_errors, 'Scoped Native adapter has failures'
            cleanup = json.loads((args.results / 'agent/cleanup.json').read_text())
            assert cleanup['cleanupQualified'] and not cleanup['errors'] and not cleanup['ambiguous']
            with lock:
                # Assignment retirement may revoke grants inside Native. Confirm
                # each exact tracked route through its authoritative retirement
                # operation before interpreting the local tracking projection.
                self.rpc.reconcile_retired_grants(json.loads((self.home / 'control/viewing.json').read_text()))
                self.rpc.retire_grants()
                self.rpc.assert_restart_clear()
                self.assert_viewing_clear()
            self.peer()
            for checkout, key in ((repo, 'harnessSourceCommit'), (args.native_source, 'nativeFixtureSourceCommit'),
                                  (args.consumer_root, 'consumerSourceCommit')):
                assert h.run('git', '-C', checkout, 'rev-parse', 'HEAD').strip() == source_pair[key], 'Frozen source changed'
            for path, key in ((args.agent_binary, 'agentBinarySha256'), (args.native_binary, 'nativeBinarySha256'),
                              (args.chrome, 'chromeSha256')):
                assert h.sha(path) == source_pair[key], 'Frozen executable changed'
            assert all(h.sha(repo / name) == digest for name, digest in source_pair['fixtureFiles'].items()), 'Frozen fixture changed'
            self.complete = True

        def independent_agent_cleanup(self):
            """Observe only; unknown/missing child cleanup is never invented."""
            path = args.results / 'agent/cleanup.json'
            if not path.exists():
                raise AssertionError('No child cleanup receipt; retained obligation requires observation')
            receipt = json.loads(path.read_text())
            process_cleanup = receipt.get('processCleanup', {})
            assert receipt.get('cleanupQualified') and process_cleanup.get('qualified')
            records = process_cleanup.get('owned', [])
            observed = []
            for record in records:
                try:
                    birth = h.birth(record['pid'])
                except (FileNotFoundError, ProcessLookupError):
                    birth = None
                assert birth != str(record['start']), 'A retained child kernel generation remains'
                observed.append({'pid': record['pid'], 'start': record['start'], 'state': 'absent' if birth is None else 'pid_reused'})
            references, denied = [], []
            markers = [str(args.results / 'agent') + '/']
            inputs_path = args.results / 'agent/inputs.json'
            if inputs_path.exists():
                inputs = json.loads(inputs_path.read_text())
                markers = [inputs['agentHome'], inputs['socketDir']]
            ancestors = {os.getpid()}
            excluded_ancestry = []
            pid = os.getpid()
            for _ in range(64):
                row = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
                excluded_ancestry.append({'pid': pid, 'start': row[19]})
                pid = int(row[1])
                if pid <= 1 or pid in ancestors:
                    break
                ancestors.add(pid)
            for proc in Path('/proc').iterdir():
                if not proc.name.isdigit() or int(proc.name) in ancestors:
                    continue
                for name in ('cmdline', 'environ'):
                    try:
                        content = (proc / name).read_bytes()
                        if any(marker.encode() in content for marker in markers):
                            references.append({'pid': int(proc.name), 'source': name})
                    except (FileNotFoundError, ProcessLookupError):
                        pass
                    except PermissionError:
                        denied.append({'pid': int(proc.name), 'source': name})
            h.private_json(args.results / 'independent-agent-cleanup.json',
                           {'generations': observed, 'references': references, 'deniedReads': denied,
                            'excludedAncestorGenerations': excluded_ancestry,
                            'qualified': not references})
            assert not references, 'Private matrix runtime references remain after Native/child teardown'

    fixture = None
    failure = cleanup_failure = None
    try:
        fixture = Acceptance(args)
        fixture.setup()
    except Exception as error:
        failure = {'type': type(error).__name__, 'stage': getattr(fixture, 'stage', 'construct'), 'message': str(error)[:500]}
        h.private_json(args.results / 'failure-trace.json', {'trace': traceback.format_exc()})
    finally:
        workload_elapsed = time.monotonic() - started
        phase_deadline[0] = final_deadline
        if fixture:
            try:
                fixture.cleanup()
            except Exception as error:
                cleanup_failure = {'type': type(error).__name__, 'message': str(error)[:500]}
            for server in fixture.extra_servers:
                try:
                    server.shutdown()
                    server.server_close()
                except Exception as error:
                    adapter_errors.append({'transport': 'listener-cleanup', 'type': type(error).__name__})
            try:
                fixture.independent_agent_cleanup()
            except Exception as error:
                adapter_errors.append({'transport': 'independent-child-cleanup', 'type': type(error).__name__})
        if socket_path.exists():
            socket_path.unlink()
        socket_root.rmdir()
    elapsed = time.monotonic() - started
    accepted = bool(fixture and fixture.complete and not failure and not cleanup_failure and not adapter_errors
                    and workload_elapsed < workload_seconds and elapsed < workload_seconds + 180)
    result = {'verdict': ('DIAGNOSTIC_ACCEPT' if args.input_diagnostic else 'ACCEPT') if accepted else 'INCOMPLETE',
              'fullMatrix': bool(accepted and not args.input_diagnostic), 'diagnosticOnly': args.input_diagnostic,
              'failure': failure, 'cleanupFailure': cleanup_failure,
              'adapterErrors': adapter_errors, 'workloadSeconds': workload_elapsed, 'elapsedSeconds': elapsed,
              'deadlineSeconds': workload_seconds, 'cleanupDeadlineSeconds': 180,
              'installationId': fixture.installation_id if fixture else None,
              'fixtureImageDisposition': 'Native fixture retains uniquely tagged build images as artifacts; no image-absence claim',
              'actualNativeInput': True if accepted and args.image_assistance else False if args.checkbox_observe_only else None,
              'challengeInputDispatched': True if accepted and args.image_assistance else False if args.checkbox_observe_only else None,
              'scope': ('Owned two-round Native image grids with actual visual selection; no full-matrix acceptance' if args.image_rounds_assistance else 'Owned static Native image round with actual agent visual selection; no full-matrix acceptance' if args.image_assistance else 'Read-only owned synthetic Native capture/rendered-state diagnosis; no checkbox acceptance' if args.checkbox_observe_only else 'Native input diagnosis only; no full-matrix acceptance' if args.input_diagnostic else
                        'One same-pair synthetic matrix on isolated Native resources; no production/account/relocation acceptance')}
    h.private_json(args.results / 'combined-matrix-receipt.json', result)
    print(json.dumps(result))
    return 0 if accepted else 1


if __name__ == '__main__':
    raise SystemExit(main())
