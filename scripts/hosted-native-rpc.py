"""Test-only Unix/HTTP Native transport, scoped by its creating fixture.

No listener or provider operation starts at import. Native remains the actual
RPC handler and authority; this module never starts a second Native owner.
"""
import json
import copy
import re
from urllib.parse import urlsplit
from urllib.request import Request, build_opener, HTTPRedirectHandler


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


class ConsumerRefused(ValueError):
    def __init__(self, value):
        super().__init__('Native authoritative consumer refusal')
        self.value = value


class ScopedNativeRpc:
    def __init__(self, control_origin, token, assignment, viewing_desktop_id,
                 application='fixture', record=lambda _event: None, embed_origin=None,
                 viewing_generation=None):
        parsed = urlsplit(control_origin)
        assert parsed.scheme == 'http' and parsed.hostname == '127.0.0.1'
        assert not parsed.path and not parsed.query and not parsed.fragment and not parsed.username
        assert token and '\r' not in token and '\n' not in token
        self.control_origin, self.token = control_origin, token
        self.assignment, self.viewing_desktop_id = dict(assignment), viewing_desktop_id
        self.viewing_generation = viewing_generation if viewing_generation is not None else assignment.get('viewingGeneration')
        if type(self.viewing_generation) is not int or self.viewing_generation < 1:
            raise ValueError('Native scope requires the observed initial viewing generation')
        self.application, self.record, self.embed_origin = application, record, embed_origin
        self.issued_grants, self.failures = {}, []
        self.published = False
        self.acquire_key = None
        self.active_rpc_count = 0
        self.forwarded_count = 0
        self.correlations = {}
        self.opener = build_opener(NoRedirect())

    @staticmethod
    def _value(response):
        if response.get('error') or response.get('result', {}).get('isError'):
            raise ValueError('Native authoritative request refused')
        return response['result']['structuredContent']

    @staticmethod
    def _response(message, value):
        return {'jsonrpc': '2.0', 'id': message.get('id'), 'result': {
            'content': [{'type': 'text', 'text': json.dumps(value)}],
            'structuredContent': value, 'isError': False}}

    def _forward(self, message):
        request = Request(self.control_origin + '/v1/mcp',
                          data=json.dumps(message).encode(), method='POST',
                          headers={'Authorization': 'Bearer ' + self.token,
                                   'Content-Type': 'application/json'})
        self.active_rpc_count += 1
        self.forwarded_count += 1
        try:
            with self.opener.open(request, timeout=120) as reply:
                body = reply.read(8 * 1024 * 1024 + 1)
                if len(body) > 8 * 1024 * 1024:
                    raise ValueError('Native response exceeds framing limit')
                response = json.loads(body)
            if 'id' in message and response.get('id') != message['id']:
                raise ValueError('Native response identity mismatch')
        except Exception:
            self.failures.append({'method': message.get('method'), 'id': message.get('id'),
                                  'outcome': 'unknown', 'retryPermitted': False})
            raise
        finally:
            self.active_rpc_count -= 1
        self.record({'method': message.get('method'), 'id': message.get('id'), 'forwarded': True})
        return response

    def handle_consumer(self, request):
        message = {'jsonrpc': '2.0', 'id': 'fixture-consumer', 'method': 'tools/call',
                   'params': {'name': 'remote_view_consumer', 'arguments': {
                       'application': self.application, 'request': request}}}
        response = self.handle(message)
        if response.get('error'):
            raise ValueError('Native consumer RPC refused')
        if response['result'].get('isError'):
            raise ConsumerRefused(response['result']['structuredContent'])
        return response['result']['structuredContent']

    def advance_viewing_fence(self, before, after):
        for value in (before, after):
            if (value.get('assignmentId') != self.assignment['assignmentId'] or
                    value.get('desktopId') != self.assignment['desktopId'] or
                    value.get('lifecycleGeneration') != self.assignment['generation'] or
                    value.get('viewingDesktopId') != self.viewing_desktop_id):
                raise ValueError('Native viewing fence cannot change its delegated target')
        if (before.get('viewingGeneration') != self.viewing_generation or
                type(after.get('viewingGeneration')) is not int or
                after['viewingGeneration'] <= self.viewing_generation):
            raise ValueError('Native viewing fence requires an independently observed advance')
        self.assert_restart_clear()
        self.viewing_generation = after['viewingGeneration']

    def assert_restart_clear(self):
        revoked = sorted(route for route, grant in self.issued_grants.items() if grant['revoked'])
        issued = sorted(self.issued_grants)
        if self.active_rpc_count or self.failures or revoked != issued:
            raise ValueError('Native restart requires confirmed grant and RPC retirement')
        return {'issuedRouteIds': issued, 'revokedRouteIds': revoked,
                'activeRpcCount': 0, 'clear': True}

    def retire_grants(self):
        # Called only by the outer creating fixture after viewer/service shutdown.
        # A refused or ambiguous revocation stops; never replay a previous intent.
        for route, grant in self.issued_grants.items():
            if not grant['revoked']:
                value = self.handle_consumer({'operation': 'revoke_inactive_view',
                                             'route_id': route, 'audience': grant['audience'],
                                             'idle_seconds': 5})
                if value.get('state') != 'revoked':
                    raise ValueError('Native grant remains active')
        return self.assert_restart_clear()

    def handle(self, message):
        before, failures = self.forwarded_count, len(self.failures)
        try:
            return self._handle(message)
        except Exception:
            if self.forwarded_count != before and len(self.failures) == failures:
                self.failures.append({'method': message.get('method'), 'id': message.get('id'),
                                      'outcome': 'unqualified_after_forward', 'retryPermitted': False})
            raise

    def _handle(self, message):
        inventory = False
        if message.get('method') == 'tools/call':
            params = message.get('params', {})
            name, arguments = params.get('name'), params.get('arguments', {})
            if name == 'remote_view_consumer':
                if arguments.get('application') != self.application:
                    raise ValueError('Native consumer scope refuses another application')
                consumer_request = arguments.get('request', {})
                if consumer_request.get('operation') == 'acquire':
                    if (consumer_request.get('pool_name') != 'main' or
                            not re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', consumer_request.get('idempotency_key', '')) or
                            set(consumer_request) - {'operation', 'pool_name', 'idempotency_key', 'presentation'}):
                        raise ValueError('Native delegation refuses invalid acquisition')
                    key = consumer_request['idempotency_key']
                    if self.acquire_key is not None and self.acquire_key != key:
                        raise ValueError('Native delegation refuses a second acquisition')
                    query = copy.deepcopy(message)
                    query['params']['arguments']['request'] = {'operation': 'inventory'}
                    value = self._value(self._forward(query))
                    matches = [item for item in value['assignments']
                               if item['assignmentId'] == self.assignment['assignmentId']]
                    if (len(matches) != 1 or matches[0]['state'] != 'active' or
                            matches[0]['desktopId'] != self.assignment['desktopId'] or
                            matches[0]['generation'] != self.assignment['generation']):
                        raise ValueError('Native delegation target is no longer active and exact')
                    self.published, self.acquire_key = True, key
                    return self._response(message, matches[0])
                operation = consumer_request.get('operation')
                if consumer_request == {'operation': 'inventory'}:
                    inventory = True
                elif operation in ('observe_assignment', 'launch_environment', 'windows', 'activate',
                                    'events', 'observe_idle', 'read_idle_retirement',
                                    'register_native_handoff_observation', 'issue_view', 'release'):
                    if (not self.published or consumer_request.get('assignment_id') != self.assignment['assignmentId'] or
                            consumer_request.get('expected_generation') != self.assignment['generation']):
                        raise ValueError('Native consumer scope refuses a peer or stale assignment')
                    if ('expected_viewing_generation' in consumer_request and
                            consumer_request['expected_viewing_generation'] != self.viewing_generation):
                        raise ValueError('Native consumer scope refuses an unobserved viewing generation')
                    if operation == 'issue_view' and (
                            not self.embed_origin or consumer_request.get('audience') != self.embed_origin or
                            consumer_request.get('capability') not in ('observe', 'control')):
                        raise ValueError('Native view scope refuses a foreign audience or capability')
                elif operation in ('resolve_view', 'revoke_view', 'revoke_inactive_view'):
                    grant = self.issued_grants.get(consumer_request.get('route_id'))
                    if not grant or consumer_request.get('audience') != grant['audience']:
                        raise ValueError('Native grant scope refuses an unknown route or audience')
                elif operation == 'observe_native_handoff_use':
                    if self.correlations.get(consumer_request.get('correlation_id')) != consumer_request.get('target_digest'):
                        raise ValueError('Native handoff scope refuses an unknown correlation')
                else:
                    raise ValueError('Native consumer scope refuses an unknown operation')
            elif name in ('remote_view_identity', 'remote_view_desktop_capabilities'):
                if arguments:
                    raise ValueError('Native metadata scope forbids target arguments')
            elif name in ('remote_view_desktop_observe', 'remote_view_desktop_capture',
                          'remote_view_desktop_capture_region', 'remote_view_desktop_window',
                          'remote_view_desktop_input', 'remote_view_desktop_control'):
                if arguments.get('desktopId') != self.viewing_desktop_id:
                    raise ValueError('Native desktop scope refuses a peer target')
                if type(arguments.get('expectedGeneration')) is not int or arguments['expectedGeneration'] != self.viewing_generation:
                    raise ValueError('Native desktop scope requires the retained viewing generation')
            else:
                raise ValueError('Native fixture scope refuses an unknown tool')
        elif message.get('method') not in ('initialize', 'notifications/initialized', 'ping'):
            raise ValueError('Native fixture scope refuses an unknown method')
        response = self._forward(message)
        if message.get('params', {}).get('name') == 'remote_view_consumer':
            request = message['params']['arguments']['request']
            result = response.get('result', {})
            if not response.get('error') and not result.get('isError'):
                value = result.get('structuredContent', {})
                if request['operation'] == 'issue_view':
                    route = value['grant']['routeId']
                    self.issued_grants[route] = {'audience': request['audience'], 'revoked': False}
                elif request['operation'] in ('revoke_view', 'revoke_inactive_view') and value.get('state') == 'revoked':
                    if value.get('routeId') != request['route_id']:
                        raise ValueError('Native revoke response target changed')
                    self.issued_grants[request['route_id']]['revoked'] = True
                elif request['operation'] == 'register_native_handoff_observation':
                    self.correlations[value['correlationId']] = request['target_digest']
        if inventory and not self.published and not response.get('error'):
            response = copy.deepcopy(response)
            result = response.get('result', {})
            if not result.get('isError'):
                def project(value):
                    value = copy.deepcopy(value)
                    value['assignments'] = [item for item in value['assignments']
                                            if item['assignmentId'] != self.assignment['assignmentId']]
                    value['desktops'] = [item for item in value['desktops']
                                        if item['desktopId'] != self.assignment['desktopId']]
                    for pool in value['pools']:
                        pool['desktopMembers'] = [item for item in pool['desktopMembers']
                                                  if item != self.assignment['desktopId']]
                    return value
                if 'structuredContent' in result:
                    result['structuredContent'] = project(result['structuredContent'])
                for item in result.get('content', []):
                    if item.get('type') == 'text':
                        item['text'] = json.dumps(project(json.loads(item['text'])))
        return response
