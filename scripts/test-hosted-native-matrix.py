"""Pure public Native viewing-store retirement checks; no runtime effects."""
import importlib.util
from pathlib import Path
import unittest


spec = importlib.util.spec_from_file_location(
    "hosted_native_matrix", Path(__file__).with_name("smoke-hosted-browser-matrix-native.py"))
matrix = importlib.util.module_from_spec(spec)
spec.loader.exec_module(matrix)


class ViewingStoreRetirementTests(unittest.TestCase):
    target = {"desktopId": "owned-desktop", "viewingDesktopId": "owned-view"}

    def test_owned_logical_route_refuses_retirement(self):
        value = {"routes": {"owned-route": {"desktopId": "owned-desktop"}},
                 "sessions": {}, "consumerGrants": {}}
        with self.assertRaises(ValueError):
            matrix.assert_viewing_store_clear(value, self.target)

    def test_owned_logical_admitted_session_refuses_retirement(self):
        value = {"routes": {}, "sessions": {
            "owned-session": {"desktopId": "owned-desktop", "state": "admitted"}},
            "consumerGrants": {}}
        with self.assertRaises(ValueError):
            matrix.assert_viewing_store_clear(value, self.target)

    def test_peer_routes_and_sessions_preserved_without_blocking_owned_retirement(self):
        value = {"routes": {"peer-route": {"desktopId": "peer-desktop"}},
                 "sessions": {"peer-session": {"desktopId": "peer-desktop", "state": "admitted"}},
                 "consumerGrants": {}}
        self.assertEqual(matrix.assert_viewing_store_clear(value, self.target),
                         {"routes": 0, "admittedSessions": 0})

    def test_live_owned_viewing_grant_refuses_retirement(self):
        value = {"routes": {}, "sessions": {}, "consumerGrants": {
            "owned-route": {"request": {"target": {"viewingDesktopId": "owned-view"}}, "revoked": False}}}
        with self.assertRaises(ValueError):
            matrix.assert_viewing_store_clear(value, self.target)


if __name__ == "__main__":
    unittest.main()
