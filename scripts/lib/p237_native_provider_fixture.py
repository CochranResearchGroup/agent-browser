"""Task-local provider boundary for source-native managed operator tests.

This is not native Remote View viewer acceptance. Production consumer code
assigns its own real managed browser to the fixture's private X11 desktop.
"""
import copy
import http.server
import http.client
import json
import subprocess
import ssl
import secrets
import threading
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


def start_native_provider_fixture(env, artifacts):
    wire_path = Path(__file__).resolve().parents[2] / 'docs/dev/contracts/remote-view-application-r3.v1.fixture.json'
    wire = json.loads(wire_path.read_text())
    operations = []
    observations = {}
    authority = artifacts / 'synthetic-xauthority'
    authority.touch(mode=0o600)
    callback_token = Path(env['HOME']) / 'p237-native-callback.token'
    callback_token.write_text(secrets.token_hex(32))
    callback_token.chmod(0o600)
    env['AGENT_BROWSER_NATIVE_HANDOFF_TOKEN_FILE'] = str(callback_token)

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            envelope = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            request = envelope['request']
            operation = request['operation']
            operations.append(operation)
            (artifacts / 'managed-provider-operations.json').write_text(json.dumps(operations) + '\n')
            if self.path != '/v1/consumer' or envelope['application'] != 'agent-browser':
                self.send_error(400)
                return
            result = None
            if operation == 'inventory':
                result = copy.deepcopy(wire['inventory'])
                result['policy']['pools']['main']['maximum_reserved'] = 1
                result['pools'][0]['capacity'] = 1
                result['desktops'][0]['viewing'] = {
                    'lifecycleGeneration': 7, 'desktopId': wire['assignmentObservation']['target']['viewingDesktopId'],
                    'generation': 19, 'publicRoute': '/1',
                }
            elif operation == 'observe_assignment':
                result = copy.deepcopy(wire['assignmentObservation'])
                generation_override = artifacts / 'managed-provider-viewing-generation.json'
                if generation_override.exists():
                    result['target']['viewingGeneration'] = json.loads(generation_override.read_text())
            elif operation == 'launch_environment':
                result = copy.deepcopy(wire['launchEnvironment'])
                result['environment'] = {'DISPLAY': env['DISPLAY'], 'XAUTHORITY': str(authority), 'REMOTE_VIEW_SLOT_GENERATION': '19'}
            elif operation == 'windows':
                result = copy.deepcopy(wire['windows'])
                result['windows'] = []
                found = subprocess.run(['xdotool', 'search', '--onlyvisible', '--class', 'Google-chrome'], env=env, capture_output=True, text=True)
                for window in found.stdout.split():
                    pid = subprocess.run(['xdotool', 'getwindowpid', window], env=env, capture_output=True, text=True)
                    if pid.returncode == 0:
                        result['windows'].append({**wire['windows']['windows'][0], 'id': int(window), 'pid': int(pid.stdout)})
            elif operation == 'register_native_handoff_observation':
                import hashlib
                key = request['idempotency_key']
                application = envelope['application']
                digest = hashlib.sha256(len(application.encode()).to_bytes(8, 'big') + application.encode() + key.encode()).hexdigest()
                record = observations.setdefault(key, {
                    'correlationId': hashlib.sha256(key.encode()).hexdigest(),
                    'requestDigest': digest, 'application': application,
                    'target': copy.deepcopy(wire['assignmentObservation']['target']),
                    'targetDigest': request['target_digest'],
                    'clock': {'bootId': 'p237-synthetic-boot', 'elapsedMs': 100},
                    'lifetimeMs': request['lifetime_ms'], 'used': None,
                })
                result = {'schemaVersion': 1, 'observation': record}
            elif operation == 'observe_native_handoff_use':
                record = next(record for record in observations.values() if record['correlationId'] == request['correlation_id'])
                result = {'schemaVersion': 1, 'state': 'pending', 'observation': record}
            elif operation == 'observe_idle':
                result = copy.deepcopy(wire['assignmentObservation'])
                result.update({'idle': False, 'viewerActive': False,
                               'retirement': {'schemaVersion': 1,
                                              'desktopId': wire['assignment']['desktopId'],
                                              'generation': wire['assignment']['generation'],
                                              'routes': [], 'sessions': []}})
            if result is None:
                self.send_error(400, 'Unsupported synthetic provider operation')
                return
            body = json.dumps(result).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    class ViewerHandler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_GET(self):
            if self.path.split('?')[0] != '/1':
                return self.proxy_dashboard()
            workspace = parse_qs(urlsplit(self.path).query).get('workspace') == ['true']
            attribute = ' data-workspace="true"' if workspace else ''
            body = f'<body id="viewer"{attribute}><p>Task-owned native viewer transport boundary</p></body>'.encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/html')
            # Mirror ordinary native viewer framing policy. A transport stub
            # must not make a blocked production iframe appear usable.
            self.send_header('Content-Security-Policy', "frame-ancestors 'self'" if workspace else "frame-ancestors 'none'")
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            self.proxy_dashboard()

        def proxy_dashboard(self):
            # A local reverse proxy supplies the shared public origin required
            # by native workspace CSP; authentication still belongs to each app.
            port = json.loads((artifacts / 'managed-dashboard-address.json').read_text())['port']
            connection = http.client.HTTPConnection('127.0.0.1', port, timeout=20)
            try:
                body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
                connection.request(self.command, self.path, body, dict(self.headers))
                reply = connection.getresponse()
                payload = reply.read()
                self.send_response(reply.status)
                for name, value in reply.getheaders():
                    if name.lower() not in ('connection', 'transfer-encoding', 'content-length'):
                        self.send_header(name, value)
                self.send_header('Content-Length', str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            finally:
                connection.close()

    cert = artifacts / 'synthetic-viewer-cert.pem'
    key = artifacts / 'synthetic-viewer-key.pem'
    subprocess.run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
                    '-keyout', str(key), '-out', str(cert), '-days', '1',
                    '-subj', '/CN=p237-native-viewer.invalid'], check=True, capture_output=True)
    key.chmod(0o600)
    viewer = http.server.ThreadingHTTPServer(('127.0.0.1', 0), ViewerHandler)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(cert, key)
    viewer.socket = tls.wrap_socket(viewer.socket, server_side=True)
    threading.Thread(target=viewer.serve_forever, daemon=True).start()
    server.viewer_fixture = viewer
    public_origin = f'https://p237-native-viewer.invalid:{viewer.server_port}'
    (artifacts / 'managed-provider-origin.json').write_text(json.dumps({'origin': f'http://127.0.0.1:{server.server_port}', 'chrome': env['AGENT_BROWSER_EXECUTABLE_PATH'], 'publicOrigin': public_origin}) + '\n')
    env.update({'AGENT_BROWSER_REMOTE_VIEW_ORIGIN': f'http://127.0.0.1:{server.server_port}',
                'AGENT_BROWSER_REMOTE_VIEW_POOL': 'main',
                'AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN': public_origin,
                'AGENT_BROWSER_REMOTE_VIEW_APPLICATION': 'agent-browser',
                'AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT': '1'})
    return server
