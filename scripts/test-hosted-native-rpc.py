"""Public scoped-RPC forwarding against an owned local HTTP boundary only."""
import copy
import importlib.util
import json
from pathlib import Path
import sys
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MODULE_PATH = Path(__file__).with_name("hosted-native-rpc.py")
spec = importlib.util.spec_from_file_location("hosted_native_rpc", MODULE_PATH)
native_rpc = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = native_rpc
spec.loader.exec_module(native_rpc)


class ScopedNativeRpcHttpTests(unittest.TestCase):
    def setUp(self):
        self.upstream_calls = []
        calls = self.upstream_calls
        self.upstream_response = {
            "jsonrpc": "2.0",
            "id": 17,
            "result": {
                "content": [{"type": "text", "text": "owned synthetic Native desktop"}],
                "structuredContent": {
                    "desktopId": "owned-view",
                    "generation": 1,
                    "state": "ready",
                    "observedValue": "actual-upstream-result",
                },
            },
        }
        response = self.upstream_response
        self.operation_responses = {}
        operation_responses = self.operation_responses

        class Upstream(BaseHTTPRequestHandler):
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                calls.append({
                    "path": self.path,
                    "authorization": self.headers.get("Authorization"),
                    "body": body,
                })
                reply = copy.deepcopy(response)
                if "id" in body:
                    reply["id"] = body["id"]
                operation = body.get("params", {}).get("arguments", {}).get("request", {}).get("operation")
                if operation in operation_responses:
                    value = copy.deepcopy(operation_responses[operation])
                    reply["result"] = {
                        "content": [{"type": "text", "text": json.dumps(value)}],
                        "structuredContent": value, "isError": False,
                    }
                encoded = json.dumps(reply).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                self.wfile.write(encoded)

            def log_message(self, *_args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.origin = f"http://127.0.0.1:{self.server.server_port}"
        self.assignment = {
            "assignmentId": "owned-assignment",
            "desktopId": "owned-desktop",
            "generation": 1,
            "viewingGeneration": 1,
        }

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        self.assertFalse(self.thread.is_alive(), "Owned HTTP fixture thread must stop")

    def test_owned_desktop_observe_forwards_real_upstream_result_and_authorization(self):
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        request = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {
                "name": "remote_view_desktop_observe",
                "arguments": {"desktopId": "owned-view", "expectedGeneration": 1},
            },
        }
        actual = rpc.handle(request)
        self.assertEqual(actual, self.upstream_response, "The real upstream result reaches the caller intact")
        self.assertEqual(len(self.upstream_calls), 1, "One public intent produces one upstream call")
        call = self.upstream_calls[0]
        self.assertEqual(call["path"], "/v1/mcp")
        self.assertEqual(call["authorization"], "Bearer fixturetoken")
        self.assertEqual(call["body"], request, "Exact owned observation arguments and request ID are preserved")


    def test_unobserved_viewing_generation_is_refused_before_upstream_mutation(self):
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        request = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {
                "name": "remote_view_desktop_window",
                "arguments": {
                    "desktopId": "owned-view", "expectedGeneration": 2,
                    "action": "activate", "windowId": 42,
                },
            },
        }
        with self.assertRaises(ValueError, msg="An unobserved viewing generation requires explicit rebind"):
            rpc.handle(request)
        self.assertEqual(self.upstream_calls, [], "The retained viewing fence must stop mutation before HTTP")

    def test_explicit_exact_viewing_fence_advance_allows_only_new_generation(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view")
        before = {"assignmentId": "owned-assignment", "desktopId": "owned-desktop",
                  "lifecycleGeneration": 1, "viewingDesktopId": "owned-view", "viewingGeneration": 1}
        after = {**before, "viewingGeneration": 2}
        rpc.advance_viewing_fence(before, after)
        request = {"jsonrpc": "2.0", "id": 17, "method": "tools/call", "params": {
            "name": "remote_view_desktop_observe",
            "arguments": {"desktopId": "owned-view", "expectedGeneration": 2}}}
        self.assertEqual(rpc.handle(request), self.upstream_response)
        self.assertEqual(len(self.upstream_calls), 1)
        request["params"]["arguments"]["expectedGeneration"] = 1
        with self.assertRaises(ValueError):
            rpc.handle(request)
        with self.assertRaises(ValueError):
            rpc.advance_viewing_fence(after, {**after, "assignmentId": "peer-assignment", "viewingGeneration": 3})
        self.assertEqual(len(self.upstream_calls), 1, "Stale requests and foreign fence advances make no HTTP calls")

    def test_authoritative_consumer_refusal_preserves_value_and_clear_restart_fence(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view")
        refused = {"schemaVersion": 1, "code": "fixture_policy_refused", "retryable": False}
        self.upstream_response["result"] = {
            "content": [{"type": "text", "text": json.dumps(refused)}],
            "structuredContent": refused, "isError": True}
        with self.assertRaises(native_rpc.ConsumerRefused) as raised:
            rpc.handle_consumer({"operation": "inventory"})
        self.assertEqual(raised.exception.value, refused)
        self.assertEqual(len(self.upstream_calls), 1)
        self.assertEqual(rpc.assert_restart_clear(), {
            "issuedRouteIds": [], "revokedRouteIds": [], "activeRpcCount": 0, "clear": True})

    def test_peer_window_activation_is_refused_before_any_upstream_call(self):
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        request = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {
                "name": "remote_view_desktop_window",
                "arguments": {
                    "desktopId": "peer-view", "expectedGeneration": 1,
                    "action": "activate", "windowId": 42,
                },
            },
        }
        with self.assertRaises(Exception, msg="Peer desktop mutation must be refused locally"):
            rpc.handle(request)
        self.assertEqual(self.upstream_calls, [], "No peer mutation may reach the HTTP boundary")


    def test_unknown_application_launch_is_refused_even_on_owned_desktop(self):
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        request = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {
                "name": "remote_view_desktop_application_launch",
                "arguments": {
                    "desktopId": "owned-view", "expectedGeneration": 1,
                    "profile": "external",
                },
            },
        }
        with self.assertRaises(Exception, msg="Unknown launch tool cannot acquire authority from an owned desktop"):
            rpc.handle(request)
        self.assertEqual(self.upstream_calls, [], "Unknown tools never reach the HTTP boundary")


    def test_consumer_inventory_reads_upstream_and_hides_only_undelegated_target(self):
        owned = {**self.assignment, "state": "active", "poolName": "main"}
        peer = {
            "assignmentId": "peer-assignment", "desktopId": "peer-desktop",
            "generation": 7, "viewingGeneration": 9, "state": "active",
            "poolName": "main", "ownerLabel": "existing-peer",
        }
        owned_desktop = {"desktopId": "owned-desktop", "generation": 1, "state": "ready"}
        peer_desktop = {"desktopId": "peer-desktop", "generation": 7, "state": "ready", "label": "Peer baseline"}
        inventory = {
            "application": "fixture", "assignments": [owned, peer],
            "desktops": [owned_desktop, peer_desktop],
            "pools": [{"poolName": "main", "desktopMembers": ["owned-desktop", "peer-desktop"], "label": "Baseline pool"}],
            "policy": {"enabled": True, "scopeMarker": "preserve-upstream-policy"},
        }
        self.upstream_response["result"] = {
            "content": [{"type": "text", "text": json.dumps(inventory)}],
            "structuredContent": inventory,
            "isError": False,
        }
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        request = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {
                "name": "remote_view_consumer",
                "arguments": {"application": "fixture", "request": {"operation": "inventory"}},
            },
        }
        actual = rpc.handle(request)
        expected_inventory = {
            **inventory, "assignments": [peer], "desktops": [peer_desktop],
            "pools": [{"poolName": "main", "desktopMembers": ["peer-desktop"], "label": "Baseline pool"}],
        }
        self.assertEqual(actual["jsonrpc"], "2.0")
        self.assertEqual(actual["id"], 17)
        self.assertFalse(actual["result"]["isError"])
        self.assertEqual(actual["result"]["structuredContent"], expected_inventory, "Keep the full peer baseline; hide only the not-yet-delegated target")
        self.assertEqual(actual["result"]["content"][0]["type"], "text")
        self.assertEqual(json.loads(actual["result"]["content"][0]["text"]), expected_inventory, "Text and structured Native inventory must agree")
        self.assertEqual(len(self.upstream_calls), 1, "Inventory is a real readonly upstream call")
        self.assertEqual(self.upstream_calls[0]["path"], "/v1/mcp")
        self.assertEqual(self.upstream_calls[0]["authorization"], "Bearer fixturetoken")
        self.assertEqual(self.upstream_calls[0]["body"], request)


    def test_acquire_delegates_actual_owned_record_after_readonly_inventory_check(self):
        # Native producer e3cf45dc44e33e590a8179b909c04c0dcad93cc4:
        # application_pool.rs AssignmentRecord; control_runtime.rs Acquire
        # returns the record directly and handle_mcp_request wraps that value.
        owned = {
            "assignmentId": "owned-assignment", "registrationId": "fixture-registration",
            "poolId": "fixture-main", "desktopId": "owned-desktop", "generation": 1,
            "state": "active", "presentation": {"userLabel": "Fresh upstream assignment"},
        }
        peer = {
            "assignmentId": "peer-assignment", "registrationId": "fixture-registration",
            "poolId": "fixture-main", "desktopId": "peer-desktop", "generation": 7,
            "state": "active", "presentation": {"userLabel": "Preserved peer"},
        }
        inventory = {
            "schemaVersion": 1,
            "assignments": [owned, peer],
            "desktops": [
                {"desktopId": "owned-desktop", "lifecycle": {"generation": 1, "state": "ready", "allocated": True}},
                {"desktopId": "peer-desktop", "lifecycle": {"generation": 7, "state": "ready", "allocated": True}},
            ],
            "pools": [{"poolId": "fixture-main", "registrationId": "fixture-registration", "name": "main", "capacity": 2, "desktopMembers": ["owned-desktop", "peer-desktop"]}],
            "policy": {"enabled": True, "scopeMarker": "unchanged-peer-baseline"},
        }
        self.upstream_response["result"] = {
            "content": [{"type": "text", "text": json.dumps(inventory)}],
            "structuredContent": inventory, "isError": False,
        }
        rpc = native_rpc.ScopedNativeRpc(
            self.origin, "fixturetoken", self.assignment, "owned-view",
            application="fixture", record=lambda _event: None,
        )
        acquire = {
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {"name": "remote_view_consumer", "arguments": {
                "application": "fixture", "request": {
                    "operation": "acquire", "pool_name": "main", "idempotency_key": "one-key",
                },
            }},
        }
        delegated = rpc.handle(acquire)
        self.assertEqual(delegated["jsonrpc"], "2.0")
        self.assertEqual(delegated["id"], 17)
        self.assertFalse(delegated["result"]["isError"])
        self.assertEqual(delegated["result"]["structuredContent"], owned, "Delegate the fresh actual Native record, not a cached constructor record or invented wrapper")
        self.assertEqual(json.loads(delegated["result"]["content"][0]["text"]), owned)
        self.assertEqual(len(self.upstream_calls), 1, "First delegation checks the real upstream inventory exactly once")
        self.assertEqual(self.upstream_calls[0]["body"]["params"]["arguments"], {
            "application": "fixture", "request": {"operation": "inventory"},
        })
        visible = rpc.handle({
            "jsonrpc": "2.0", "id": 17, "method": "tools/call",
            "params": {"name": "remote_view_consumer", "arguments": {
                "application": "fixture", "request": {"operation": "inventory"},
            }},
        })
        self.assertEqual(visible["result"]["structuredContent"], inventory, "Published target and full peer metadata remain actual upstream data")
        self.assertEqual(json.loads(visible["result"]["content"][0]["text"]), inventory)
        self.assertEqual(len(self.upstream_calls), 2)
        for call in self.upstream_calls:
            self.assertEqual(call["path"], "/v1/mcp")
            self.assertEqual(call["authorization"], "Bearer fixturetoken")
            self.assertEqual(call["body"]["params"]["arguments"]["request"], {"operation": "inventory"}, "No acquire may be forwarded to Native")


    def delegate_owned_through_public_acquire(self, rpc):
        owned = {
            "assignmentId": "owned-assignment", "registrationId": "fixture-registration",
            "poolId": "fixture-main", "desktopId": "owned-desktop", "generation": 1,
            "state": "active",
        }
        self.operation_responses["inventory"] = {
            "assignments": [owned],
            "desktops": [{"desktopId": "owned-desktop", "lifecycle": {"generation": 1, "state": "ready"}}],
            "pools": [{"poolId": "fixture-main", "name": "main", "desktopMembers": ["owned-desktop"]}],
        }
        return rpc.handle_consumer({"operation": "acquire", "pool_name": "main", "idempotency_key": "one-key"})

    def owned_view_target(self):
        # Native e3cf45d viewing.rs ConsumerViewTarget, camelCase response.
        return {
            "assignmentId": "owned-assignment", "registrationId": "fixture-registration",
            "desktopId": "owned-desktop", "lifecycleGeneration": 1,
            "viewingDesktopId": "owned-view", "viewingGeneration": 1,
        }

    def test_raw_consumer_owned_observation_forwards_actual_native_target(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view", application="fixture")
        self.delegate_owned_through_public_acquire(rpc)
        expected = {"schemaVersion": 1, "target": self.owned_view_target(), "readinessScope": "live_resource"}
        self.operation_responses["observe_assignment"] = expected
        request = {"operation": "observe_assignment", "assignment_id": "owned-assignment", "expected_generation": 1}
        actual = rpc.handle_consumer(request)
        self.assertEqual(actual, expected, "Raw consumer API returns the actual Native target, not the MCP envelope")
        self.assertEqual(len(self.upstream_calls), 2)
        self.assertEqual(self.upstream_calls[-1]["body"]["params"]["arguments"], {"application": "fixture", "request": request})
        self.assertEqual(self.upstream_calls[-1]["authorization"], "Bearer fixturetoken")

    def test_peer_consumer_target_operations_are_refused_before_http(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view", application="fixture")
        requests = [
            {"operation": "launch_environment", "assignment_id": "peer-assignment", "expected_generation": 7, "expected_viewing_generation": 9},
            {"operation": "issue_view", "assignment_id": "peer-assignment", "expected_generation": 7, "expected_viewing_generation": 9,
             "audience": "fixture", "capability": "control", "lifetime_seconds": 60, "idempotency_key": "peer-view"},
            {"operation": "release", "assignment_id": "peer-assignment", "expected_generation": 7, "idempotency_key": "peer-release",
             "cleanup": {"schemaVersion": 1, "assignmentId": "peer-assignment", "lifecycleGeneration": 7,
                         "applicationReferencesClear": True, "pendingRecoveryClear": True, "foregroundLeasesClear": True, "cleanupTasksClear": True}},
        ]
        for request in requests:
            with self.subTest(operation=request["operation"]):
                with self.assertRaises(ValueError):
                    rpc.handle_consumer(request)
                self.assertEqual(self.upstream_calls, [], "Peer consumer operations cannot cross the HTTP boundary")

    def test_wrong_consumer_application_is_refused_before_http(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view", application="fixture")
        with self.assertRaises(ValueError):
            rpc.handle({"jsonrpc": "2.0", "id": 17, "method": "tools/call", "params": {
                "name": "remote_view_consumer", "arguments": {"application": "peer-application", "request": {"operation": "inventory"}},
            }})
        self.assertEqual(self.upstream_calls, [])

    def issue_owned_observe_grant(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view",
                                        application="fixture", embed_origin="https://fixture.example.test")
        self.delegate_owned_through_public_acquire(rpc)
        issued_at = int(time.time() * 1000)
        grant = {"schemaVersion": 1, "routeId": "owned-observe-route", "request": {
            "idempotencyKey": "fixture:observe-key", "target": self.owned_view_target(),
            "application": "fixture", "audience": "https://fixture.example.test", "capability": "observe",
            "policyRevision": "fixture-policy-v1", "lifetimeSeconds": 60},
            "issuedAt": issued_at, "expiresAt": issued_at + 60_000, "revoked": False}
        self.operation_responses["issue_view"] = {"schemaVersion": 1, "grant": grant,
            "path": "/embed/owned-observe-route", "presentationState": "grant_issued", "readinessScope": "live_resource"}
        rpc.handle_consumer({"operation": "issue_view", "assignment_id": "owned-assignment", "expected_generation": 1,
            "expected_viewing_generation": 1, "audience": "https://fixture.example.test", "capability": "observe",
            "lifetime_seconds": 60, "idempotency_key": "observe-key"})
        return rpc, grant

    def test_observe_grant_retirement_uses_exact_revoke_view(self):
        rpc, _grant = self.issue_owned_observe_grant()
        self.operation_responses["revoke_view"] = {"schemaVersion": 1, "routeId": "owned-observe-route", "state": "revoked"}
        self.assertTrue(rpc.retire_grants()["clear"])
        operations = [call["body"]["params"]["arguments"]["request"]["operation"] for call in self.upstream_calls]
        self.assertEqual(operations, ["inventory", "issue_view", "revoke_view"])

    def test_reconcile_authoritatively_retired_exact_grant_clears_projection(self):
        rpc, grant = self.issue_owned_observe_grant()
        viewing = {"consumerGrants": {grant["routeId"]: {**grant, "revoked": True}}, "routes": {}, "sessions": {}}
        rpc.reconcile_retired_grants(viewing)
        self.assertEqual(rpc.assert_restart_clear()["revokedRouteIds"], [grant["routeId"]])
        self.assertEqual(len(self.upstream_calls), 2, "Read-only reconciliation dispatches no revocation")

    def test_reconcile_refuses_foreign_binding_or_still_admitted_route(self):
        rpc, grant = self.issue_owned_observe_grant()
        retired = {**grant, "revoked": True}
        foreign = copy.deepcopy(retired)
        foreign["request"]["target"]["assignmentId"] = "peer-assignment"
        for viewing in (
            {"consumerGrants": {grant["routeId"]: foreign}, "routes": {}, "sessions": {}},
            {"consumerGrants": {grant["routeId"]: retired},
             "routes": {grant["routeId"]: {"desktopId": "owned-desktop"}}, "sessions": {}},
        ):
            with self.subTest(viewing=viewing), self.assertRaises(ValueError):
                rpc.reconcile_retired_grants(viewing)
            with self.assertRaises(ValueError):
                rpc.assert_restart_clear()
        self.assertEqual(len(self.upstream_calls), 2)

    def test_issued_native_grant_blocks_restart_until_exact_authoritative_revocation(self):
        rpc = native_rpc.ScopedNativeRpc(self.origin, "fixturetoken", self.assignment, "owned-view", application="fixture", embed_origin="https://fixture.example.test")
        self.delegate_owned_through_public_acquire(rpc)
        issued_at = int(time.time() * 1000)
        # Pinned Native IssueView returns {grant, path}; ConsumerViewerGrant has
        # revoked:false. RevokeView returns state:"revoked", not a boolean flag.
        grant = {
            "schemaVersion": 1, "routeId": "owned-route",
            "request": {"idempotencyKey": "fixture:issue-key", "target": self.owned_view_target(), "application": "fixture",
                        "audience": "https://fixture.example.test", "capability": "control", "policyRevision": "fixture-policy-v1", "lifetimeSeconds": 60},
            "issuedAt": issued_at, "expiresAt": issued_at + 60_000, "revoked": False,
        }
        issued = {"schemaVersion": 1, "grant": grant, "path": "/embed/owned-route", "presentationState": "grant_issued", "readinessScope": "live_resource"}
        self.operation_responses["issue_view"] = issued
        actual = rpc.handle_consumer({"operation": "issue_view", "assignment_id": "owned-assignment", "expected_generation": 1,
                                      "expected_viewing_generation": 1, "audience": "https://fixture.example.test", "capability": "control",
                                      "lifetime_seconds": 60, "idempotency_key": "issue-key"})
        self.assertEqual(actual, issued)
        with self.assertRaises(ValueError, msg="A live issued grant keeps the restart fence held"):
            rpc.assert_restart_clear()
        revoked = {"schemaVersion": 1, "routeId": "owned-route", "state": "revoked"}
        self.operation_responses["revoke_view"] = revoked
        self.assertEqual(rpc.handle_consumer({"operation": "revoke_view", "route_id": "owned-route", "audience": "https://fixture.example.test"}), revoked)
        clear = rpc.assert_restart_clear()
        self.assertTrue(clear["clear"])
        self.assertEqual(clear["issuedRouteIds"], ["owned-route"])
        self.assertEqual(clear["revokedRouteIds"], ["owned-route"])
        self.assertEqual(clear["activeRpcCount"], 0)
        operations = [call["body"]["params"]["arguments"]["request"]["operation"] for call in self.upstream_calls]
        self.assertEqual(operations, ["inventory", "issue_view", "revoke_view"], "Tracking does not synthesize another Native acquisition or revocation")


if __name__ == "__main__":
    unittest.main()
